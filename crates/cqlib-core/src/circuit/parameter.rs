// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

//! Symbolic and numeric parameter type for quantum circuit gate angles.
//!
//! [`Parameter`] wraps a [`symb_anafis::Expr`] expression tree, allowing gate parameters
//! to be either concrete floating-point numbers or symbolic expressions containing
//! free variables (e.g. `θ`, `φ`).  Symbolic parameters can be composed with
//! arithmetic operators, differentiated, simplified, and finally evaluated by
//! supplying variable bindings.
//!
//! # Quick start
//!
//! ```rust
//! use cqlib_core::circuit::Parameter;
//!
//! // Numeric parameter
//! let angle = Parameter::from(std::f64::consts::PI / 2.0);
//!
//! // Symbolic parameter
//! let theta = Parameter::symbol("θ");
//! let expr  = theta.clone() * Parameter::from(2.0) + Parameter::pi();
//!
//! // Evaluate with a concrete binding
//! let val = expr.evaluate(&Some([("θ", 0.5)].iter().cloned().collect())).unwrap();
//! ```

use crate::circuit::error::ParameterError;
use core::fmt::Debug;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};
use std::str::FromStr;
use symb_anafis::{Expr, Simplify, parse, symb, visitor::ExprView};

// The engine's map() treats polynomials as opaque and visits rebuilt nodes.
// Walk the public view instead: it exposes polynomial terms, and replacement
// expressions are returned immediately without being traversed themselves.
fn substitute_parameter_expr<'a>(expr: &Expr, lookup: &impl Fn(&str) -> Option<&'a Expr>) -> Expr {
    match expr.view() {
        ExprView::Number(_) => expr.clone(),
        ExprView::Symbol(name) => lookup(&name).cloned().unwrap_or_else(|| expr.clone()),
        ExprView::Sum(children) | ExprView::Product(children) => {
            let mapped: Vec<_> = children
                .iter()
                .map(|child| substitute_parameter_expr(child, lookup))
                .collect();
            if children.iter().zip(&mapped).all(|(a, b)| a.as_ref() == b) {
                return expr.clone();
            }

            // Smart sum/product constructors can prune tiny coefficients.
            // Construct a template of distinct function calls first, then use
            // map() only on that template, whose children have no polynomials.
            // Inserted expressions never pass through the smart constructors.
            let slots = (0..mapped.len())
                .map(|i| Expr::func("__cqlib_substitution_slot", Expr::number(i as f64)))
                .collect();
            let template = if matches!(expr.view(), ExprView::Sum(_)) {
                Expr::sum(slots)
            } else {
                Expr::product(slots)
            };
            let slots = match template.view() {
                ExprView::Sum(slots) | ExprView::Product(slots) => slots.into_owned(),
                _ => return mapped.into_iter().next().unwrap_or(template),
            };
            // Match in template order so reconstruction preserves the original
            // child order even if construction sorted the placeholder nodes.
            let replacements: HashMap<_, _> = slots
                .into_iter()
                .map(|slot| (*slot).clone())
                .zip(mapped)
                .collect();
            template.map(|node| {
                replacements
                    .get(node)
                    .cloned()
                    .unwrap_or_else(|| node.clone())
            })
        }
        ExprView::Div(lhs, rhs) => {
            let a = substitute_parameter_expr(lhs, lookup);
            let b = substitute_parameter_expr(rhs, lookup);
            if &a == lhs && &b == rhs {
                expr.clone()
            } else {
                div_parameter_expr(a, b)
            }
        }
        ExprView::Pow(base, exponent) => {
            let a = substitute_parameter_expr(base, lookup);
            let b = substitute_parameter_expr(exponent, lookup);
            if &a == base && &b == exponent {
                expr.clone()
            } else {
                Expr::pow_static(a, b)
            }
        }
        ExprView::Function { name, args } => {
            let mapped: Vec<_> = args
                .iter()
                .map(|arg| substitute_parameter_expr(arg, lookup))
                .collect();
            if args.iter().zip(&mapped).all(|(a, b)| a.as_ref() == b) {
                expr.clone()
            } else {
                Expr::func_multi(name, mapped)
            }
        }
        ExprView::Derivative { inner, var, order } => {
            let mapped = substitute_parameter_expr(inner, lookup);
            if &mapped == inner {
                expr.clone()
            } else {
                Expr::derivative(mapped, var, order)
            }
        }
    }
}

// Integral coefficients in the exactly representable integer range cannot
// trigger the engine's near-zero/near-one pruning. Ordinary sums of symbols
// therefore avoid coefficient collection and expression rebuilding entirely.
fn needs_affine_protection(expr: &Expr) -> bool {
    match expr.view() {
        ExprView::Number(value) => value.fract() != 0.0 || value.abs() > (1u64 << 53) as f64,
        ExprView::Sum(children) | ExprView::Product(children) => {
            children.iter().any(|child| needs_affine_protection(child))
        }
        ExprView::Div(numerator, denominator) => {
            needs_affine_protection(numerator)
                || denominator.as_number().is_some_and(|value| {
                    value != 0.0 && needs_affine_protection(&Expr::number(1.0 / value))
                })
        }
        _ => false,
    }
}

type AffineCoefficients = HashMap<Option<Expr>, f64>;

// Coefficients are collected once per input, using hashed symbol keys. Building
// a replacement AST is deferred until the engine actually changes a coefficient.
fn affine_coefficients(inputs: &[(&Expr, f64)]) -> Option<AffineCoefficients> {
    fn collect(expr: &Expr, scale: f64, terms: &mut AffineCoefficients) -> Option<()> {
        match expr.view() {
            ExprView::Number(value) => *terms.entry(None).or_default() += scale * value,
            ExprView::Symbol(_) => *terms.entry(Some(expr.clone())).or_default() += scale,
            ExprView::Sum(children) => {
                for child in children.iter() {
                    collect(child, scale, terms)?;
                }
            }
            ExprView::Product(children) => {
                let mut scale = scale;
                let mut symbolic = None;
                for child in children.iter() {
                    if let Some(value) = child.as_number() {
                        scale *= value;
                    } else if symbolic.replace(child).is_some() {
                        return None;
                    }
                }
                if let Some(child) = symbolic {
                    collect(child, scale, terms)?;
                } else {
                    *terms.entry(None).or_default() += scale;
                }
            }
            ExprView::Div(numerator, denominator) => {
                let divisor = denominator.as_number()?;
                if divisor == 0.0 {
                    return None;
                }
                collect(numerator, scale / divisor, terms)?;
            }
            _ => return None,
        }
        Some(())
    }

    let mut terms = HashMap::new();
    for (expr, scale) in inputs {
        collect(expr, *scale, &mut terms)?;
    }
    if terms.values().any(|coefficient| !coefficient.is_finite()) {
        return None;
    }
    terms.retain(|_, coefficient| *coefficient != 0.0);
    Some(terms)
}

fn weighted_symbol(symbol: Expr, coefficient: f64) -> Expr {
    if coefficient == 0.0 {
        return Expr::number(0.0);
    }
    if coefficient == 1.0 {
        return symbol;
    }
    let product = Expr::product(vec![Expr::number(coefficient), symbol.clone()]);
    if let ExprView::Product(factors) = product.view()
        && factors
            .iter()
            .any(|factor| factor.as_number() == Some(coefficient))
    {
        return product;
    }
    // Public map() reconstructs products without approximate numeric pruning.
    // If construction rounded a near-unit coefficient, insert its exact value
    // into a non-unit product template after construction.
    Expr::product(vec![Expr::number(2.0), symbol]).map(|factor| {
        if factor.as_number() == Some(2.0) {
            Expr::number(coefficient)
        } else {
            factor.clone()
        }
    })
}

fn affine_expr(terms: &AffineCoefficients) -> Expr {
    // Construct a sum of distinct unit-weight symbols before inserting weights,
    // preventing automatic polynomial conversion from deleting small terms.
    let unit_sum = Expr::sum(
        terms
            .keys()
            .map(|symbol| symbol.clone().unwrap_or_else(|| Expr::number(1.0)))
            .collect(),
    );
    unit_sum.map(|node| {
        if node.as_number() == Some(1.0) {
            Expr::number(terms.get(&None).copied().unwrap_or(1.0))
        } else if matches!(node.view(), ExprView::Symbol(_)) {
            weighted_symbol(
                node.clone(),
                terms.get(&Some(node.clone())).copied().unwrap_or(1.0),
            )
        } else {
            node.clone()
        }
    })
}

fn preserve_affine(expected: Option<AffineCoefficients>, fallback: Expr) -> Expr {
    match expected {
        Some(expected) if expected.keys().all(Option::is_none) => {
            Expr::number(expected.get(&None).copied().unwrap_or(0.0))
        }
        Some(expected) if affine_coefficients(&[(&fallback, 1.0)]).as_ref() != Some(&expected) => {
            affine_expr(&expected)
        }
        _ => fallback,
    }
}

fn add_parameter_expr(lhs: Expr, rhs: Expr) -> Expr {
    if let (Some(a), Some(b)) = (lhs.as_number(), rhs.as_number()) {
        return Expr::number(a + b);
    }
    if !needs_affine_protection(&lhs) && !needs_affine_protection(&rhs) {
        return lhs + rhs;
    }
    let affine = affine_coefficients(&[(&lhs, 1.0), (&rhs, 1.0)]);
    preserve_affine(affine, lhs + rhs)
}

fn sub_parameter_expr(lhs: Expr, rhs: Expr) -> Expr {
    if let (Some(a), Some(b)) = (lhs.as_number(), rhs.as_number()) {
        return Expr::number(a - b);
    }
    if !needs_affine_protection(&lhs) && !needs_affine_protection(&rhs) {
        return lhs - rhs;
    }
    let affine = affine_coefficients(&[(&lhs, 1.0), (&rhs, -1.0)]);
    preserve_affine(affine, lhs - rhs)
}

// The common rotation-angle case needs neither a coefficient map nor a second
// symbolic operation. Handle one weighted symbol directly, including rescaling.
fn scaled_symbol(expr: &Expr, scale: f64) -> Option<Expr> {
    let (symbol, coefficient) = match expr.view() {
        ExprView::Symbol(_) => (expr.clone(), scale),
        ExprView::Product(children) if children.len() == 2 => {
            let (number, symbol) = if let Some(number) = children[0].as_number() {
                (number, &children[1])
            } else {
                (children[1].as_number()?, &children[0])
            };
            if !matches!(symbol.view(), ExprView::Symbol(_)) {
                return None;
            }
            ((**symbol).clone(), number * scale)
        }
        _ => return None,
    };
    coefficient
        .is_finite()
        .then(|| weighted_symbol(symbol, coefficient))
}

fn mul_parameter_expr(lhs: Expr, rhs: Expr) -> Expr {
    if let (Some(a), Some(b)) = (lhs.as_number(), rhs.as_number()) {
        return Expr::number(a * b);
    }
    let affine = if let Some(scale) = lhs.as_number() {
        if let Some(expr) = scaled_symbol(&rhs, scale) {
            return expr;
        }
        if !needs_affine_protection(&lhs) && !needs_affine_protection(&rhs) {
            return lhs * rhs;
        }
        affine_coefficients(&[(&rhs, scale)])
    } else if let Some(scale) = rhs.as_number() {
        if let Some(expr) = scaled_symbol(&lhs, scale) {
            return expr;
        }
        if !needs_affine_protection(&lhs) && !needs_affine_protection(&rhs) {
            return lhs * rhs;
        }
        affine_coefficients(&[(&lhs, scale)])
    } else {
        None
    };
    preserve_affine(affine, lhs * rhs)
}

fn div_parameter_expr(lhs: Expr, rhs: Expr) -> Expr {
    if let (Some(a), Some(b)) = (lhs.as_number(), rhs.as_number())
        && b != 0.0
    {
        return Expr::number(a / b);
    }
    let affine = if let Some(divisor) = rhs.as_number().filter(|value| *value != 0.0) {
        affine_coefficients(&[(&lhs, 1.0)]).and_then(|mut terms| {
            // Divide coefficients directly: the reciprocal of a subnormal
            // divisor can overflow even when the quotient is representable.
            for coefficient in terms.values_mut() {
                *coefficient /= divisor;
            }
            if terms.values().any(|coefficient| !coefficient.is_finite()) {
                return None;
            }
            terms.retain(|_, coefficient| *coefficient != 0.0);
            Some(terms)
        })
    } else {
        None
    };
    preserve_affine(affine, lhs / rhs)
}

/// A symbolic or numeric parameter used to represent gate angles and other
/// continuous values in a quantum circuit.
///
/// `Parameter` is a thin, cheaply-cloneable wrapper around [`symb_anafis::Expr`].
/// It supports the full suite of arithmetic operators (`+`, `-`, `*`, `/`)
/// between `Parameter` values as well as between `Parameter` and primitive numeric types
/// (`f64`, `f32`, `i32`, `u32`).
///
/// The mathematical constants `π` and `e` are always available as free
/// symbols and are automatically resolved during [`Parameter::evaluate`].
#[derive(Clone, Debug)]
pub struct Parameter {
    /// The root node of the mathematical expression tree.
    expr: Expr,
}

impl From<Expr> for Parameter {
    fn from(expr: Expr) -> Self {
        Parameter { expr }
    }
}

impl From<Parameter> for Expr {
    fn from(p: Parameter) -> Self {
        p.expr
    }
}

impl fmt::Display for Parameter {
    /// Formats the parameter as a human-readable mathematical string.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.expr, f)
    }
}

impl Default for Parameter {
    /// Creates a default parameter representing the integer `0`.
    fn default() -> Self {
        Self {
            expr: Expr::from(0),
        }
    }
}

impl std::hash::Hash for Parameter {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.expr.hash(state);
    }
}

impl PartialEq for Parameter {
    /// Checks equality based on the underlying expression structure.
    ///
    /// Note: This performs **structural** equality of the AST, not semantic
    /// mathematical equality.  For example, `x + y` and `y + x` are
    /// structurally different and will compare as unequal.
    fn eq(&self, other: &Self) -> bool {
        self.expr == other.expr
    }
}

impl Eq for Parameter {}

impl TryFrom<&str> for Parameter {
    type Error = ParameterError;

    /// Parses a symbolic expression from a string slice.
    ///
    /// Returns [`ParameterError::ParseError`] if the string cannot be parsed
    /// as a valid expression.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cqlib_core::circuit::Parameter;
    ///
    /// let p = Parameter::try_from("θ * 2 + π").unwrap();
    /// let bad = Parameter::try_from("@@@");
    /// assert!(bad.is_err());
    /// ```
    fn try_from(expr: &str) -> Result<Self, Self::Error> {
        if let Ok(expr) = parse(expr, &HashSet::new(), &HashSet::new(), None) {
            Ok(Self { expr })
        } else {
            Err(ParameterError::ParseError(expr.to_string()))
        }
    }
}

impl FromStr for Parameter {
    type Err = ParameterError;

    /// Parses a symbolic expression from a string.
    ///
    /// Returns [`ParameterError::ParseError`] if the string cannot be parsed
    /// as a valid expression.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cqlib_core::circuit::Parameter;
    /// use std::str::FromStr;
    ///
    /// let p = Parameter::from_str("θ * 2 + π").unwrap();
    /// let bad = Parameter::from_str("@@@");
    /// assert!(bad.is_err());
    /// ```
    fn from_str(expr: &str) -> Result<Self, Self::Err> {
        Self::try_from(expr)
    }
}

impl Parameter {
    /// Creates a new `Parameter` directly from a [`symb_anafis::Expr`] node.
    ///
    /// Prefer [`Parameter::symbol`] for named variables or the `From<f64>` / `From<i32>`
    /// impls for numeric constants.
    pub fn new(expr: Expr) -> Self {
        Self { expr }
    }

    /// Returns a reference to the underlying expression.
    pub fn as_expr(&self) -> &Expr {
        &self.expr
    }

    /// Consumes the parameter and returns the underlying expression.
    pub fn into_expr(self) -> Expr {
        self.expr
    }

    /// Creates a symbolic parameter representing a free variable with the
    /// given `name`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cqlib_core::circuit::Parameter;
    ///
    /// let theta = Parameter::symbol("θ");
    /// assert_eq!(theta.get_symbols().contains("θ"), true);
    /// ```
    pub fn symbol(name: &str) -> Self {
        Self::new(Expr::from(symb(name)))
    }

    /// Evaluates the expression to a concrete `f64` value.
    ///
    /// The mathematical constants `π` and `e` are injected automatically, so
    /// callers do not need to supply them.  Additional variable bindings are
    /// provided through `bindings`.
    ///
    /// Returns [`ParameterError::NaN`] if the expression still contains
    /// unbound free variables after applying the supplied bindings.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cqlib_core::circuit::Parameter;
    /// use std::collections::HashMap;
    ///
    /// // Numeric constant — no bindings needed.
    /// let p = Parameter::from(std::f64::consts::PI);
    /// assert!((p.evaluate(&None).unwrap() - std::f64::consts::PI).abs() < 1e-10);
    ///
    /// // Symbolic expression with a user-supplied binding.
    /// let theta = Parameter::symbol("θ");
    /// let expr  = theta * Parameter::from(2.0);
    /// let mut bindings = HashMap::new();
    /// bindings.insert("θ", 1.0_f64);
    /// assert!((expr.evaluate(&Some(bindings)).unwrap() - 2.0).abs() < 1e-10);
    /// ```
    pub fn evaluate(&self, bindings: &Option<HashMap<&str, f64>>) -> Result<f64, ParameterError> {
        // Evaluate directly first. Numeric expressions and fully-bound user
        // expressions avoid constructing a merged binding table entirely.
        let mut evaluated = match bindings {
            Some(bindings) => self.expr.evaluate(bindings, &HashMap::new()),
            None => self.expr.evaluate(&(), &HashMap::new()),
        };
        // `symb_anafis` recognizes ASCII `pi`, while `Parameter::pi` uses the
        // public Unicode spelling `π`. Only retry unresolved expressions with
        // the compatibility constants, preserving the historical ability for
        // explicit caller bindings to override them.
        if evaluated.as_number().is_none() {
            let mut actual_bindings =
                HashMap::with_capacity(bindings.as_ref().map_or(2, |bindings| bindings.len() + 2));
            actual_bindings.insert("π", std::f64::consts::PI);
            actual_bindings.insert("e", std::f64::consts::E);
            if let Some(bindings) = bindings {
                actual_bindings.extend(bindings);
            }
            evaluated = self.expr.evaluate(&actual_bindings, &HashMap::new());
        }

        if let Some(n) = evaluated.as_number() {
            // Check for NaN
            if n.is_nan() {
                return Err(ParameterError::DomainError(format!(
                    "Evaluation of '{}' resulted in NaN",
                    self.expr
                )));
            }
            // Check for infinity (positive or negative)
            if n.is_infinite() {
                return Err(ParameterError::DomainError(format!(
                    "Evaluation of '{}' resulted in infinity",
                    self.expr
                )));
            }
            Ok(n)
        } else {
            Err(ParameterError::NaN(self.expr.to_string()))
        }
    }

    /// Returns an algebraically simplified form of this parameter.
    ///
    /// Simplification is performed in a domain-safe mode to avoid invalid
    /// transformations (e.g. taking the log of a potentially negative number).
    /// For affine sums, scalar products and division by numeric constants,
    /// nonzero `f64` coefficients and cancellation results are preserved without
    /// tolerance pruning. Numeric literals are retained exactly. Floating-point
    /// rounding, overflow and underflow still apply. Parsing and general
    /// nonlinear simplification use the symbolic engine's own numeric policy.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cqlib_core::circuit::Parameter;
    ///
    /// let theta = Parameter::symbol("θ");
    /// let expr  = theta.clone() * Parameter::from(1.0);   // θ * 1
    /// let simplified = expr.simplify().unwrap();
    /// assert_eq!(simplified.to_string(), "θ");
    /// ```
    pub fn simplify(&self) -> Result<Self, ParameterError> {
        if self.expr.as_number().is_some() {
            return Ok(self.clone());
        }
        let affine = needs_affine_protection(&self.expr)
            .then(|| affine_coefficients(&[(&self.expr, 1.0)]))
            .flatten();
        let expr = Simplify::new().domain_safe(true).simplify(&self.expr)?;
        Ok(Self {
            expr: preserve_affine(affine, expr),
        })
    }

    /// Returns the numeric value of this parameter without symbolic
    /// simplification, if it can be determined directly.
    ///
    /// Plain number nodes are read at zero cost; other constant expressions
    /// (e.g. `pi/2`) fall back to evaluation.  Non-finite values and
    /// expressions with unbound symbols return `None`.
    fn numeric_value(&self) -> Option<f64> {
        self.expr
            .as_number()
            .filter(|n| n.is_finite())
            .or_else(|| self.evaluate(&None).ok())
    }

    /// Returns the canonical storage form of this parameter expression.
    ///
    /// A constant expression is evaluated before symbolic simplification. If
    /// simplification eliminates all symbols, the result is also evaluated and
    /// rebuilt as a finite numeric parameter. This also normalizes `-0.0` to
    /// `0.0`, giving circuit operation storage a stable representation for
    /// equivalent constants.
    ///
    /// Symbolic expressions are returned in their simplified form. This method
    /// does not intern symbolic parameters into any circuit; use
    /// [`Circuit::map_param`](crate::circuit::Circuit::map_param) when a
    /// [`CircuitParam`](crate::circuit::CircuitParam) is needed for operation
    /// storage.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError`] if simplification fails, a constant expression
    /// cannot be evaluated, or the evaluated constant is not finite.
    pub fn canonicalized(&self) -> Result<Self, ParameterError> {
        // Evaluate constant expressions before any approximate symbolic rewrite.
        if self.get_symbols().is_empty() {
            let value = self.evaluate(&None)?;
            return Ok(Parameter::from(if value == 0.0 { 0.0 } else { value }));
        }
        let simplified = self.simplify()?;

        if simplified.get_symbols().is_empty() {
            let value = simplified.evaluate(&None)?;
            if !value.is_finite() {
                return Err(ParameterError::DomainError(format!(
                    "constant parameter evaluates to non-finite value {value}"
                )));
            }
            let value = if value == 0.0 { 0.0 } else { value };
            Ok(Parameter::from(value))
        } else {
            Ok(simplified)
        }
    }

    /// Returns whether this parameter is exactly the numeric constant `0.0`.
    ///
    /// This is stricter than [`Parameter::is_zero`]: symbolic parameters return
    /// `false`, and constants are compared exactly after evaluation rather than
    /// with an epsilon tolerance. Compiler canonicalization uses this predicate
    /// when deciding whether a no-op is represented exactly as zero.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError`] if a constant parameter cannot be evaluated.
    pub fn is_exact_zero(&self) -> Result<bool, ParameterError> {
        if self.get_symbols().is_empty() {
            Ok(self.evaluate(&None)? == 0.0)
        } else {
            Ok(false)
        }
    }

    /// Returns whether this is already the canonical numeric `+0.0` node.
    ///
    /// Unlike [`Parameter::is_exact_zero`], this does not evaluate constant
    /// expressions and deliberately rejects `-0.0`. Compiler hot paths use it
    /// only to skip work that is provably an exact representation no-op.
    pub(crate) fn is_canonical_positive_zero(&self) -> bool {
        self.expr
            .as_number()
            .is_some_and(|value| value.to_bits() == 0.0_f64.to_bits())
    }

    /// Computes the symbolic partial derivative of this expression with
    /// respect to the variable `var`.
    ///
    /// Returns [`ParameterError`] if differentiation is not supported for
    /// the expression.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cqlib_core::circuit::Parameter;
    ///
    /// let theta = Parameter::symbol("θ");
    /// let expr  = theta.clone() * theta.clone();  // θ²
    /// let deriv = expr.derivative("θ").unwrap();  // 2θ
    /// let val   = deriv.evaluate(&Some([("θ", 3.0)].iter().cloned().collect())).unwrap();
    /// assert!((val - 6.0).abs() < 1e-10);
    /// ```
    pub fn derivative(&self, var: &str) -> Result<Self, ParameterError> {
        let expr = self.expr.diff(var)?;
        Ok(Self { expr })
    }

    /// Returns the set of free variable names present in this expression.
    ///
    /// The built-in constants `π` and `e` may appear here if the expression
    /// was constructed symbolically (e.g. via [`Parameter::pi`]), but they are
    /// resolved automatically during [`Parameter::evaluate`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cqlib_core::circuit::Parameter;
    ///
    /// let expr = Parameter::symbol("θ") + Parameter::symbol("φ");
    /// let syms = expr.get_symbols();
    /// assert!(syms.contains("θ"));
    /// assert!(syms.contains("φ"));
    /// ```
    pub fn get_symbols(&self) -> HashSet<String> {
        let mut symbols = self.expr.variables();
        symbols.remove("π");
        symbols.remove("pi");
        symbols.remove("e");
        symbols
    }

    /// Returns a new parameter representing the absolute value `|x|`.
    pub fn abs(&self) -> Self {
        Self {
            expr: self.expr.clone().abs(),
        }
    }

    /// Returns a new parameter representing the square root `√x`.
    pub fn sqrt(&self) -> Self {
        Self {
            expr: self.expr.clone().sqrt(),
        }
    }

    /// Returns a new parameter representing the exponential `eˣ`.
    pub fn exp(&self) -> Self {
        Self {
            expr: self.expr.clone().exp(),
        }
    }

    /// Returns a new parameter representing the sine `sin(x)`.
    pub fn sin(&self) -> Self {
        Self {
            expr: self.expr.clone().sin(),
        }
    }

    /// Returns a new parameter representing the inverse sine `asin(x)`.
    pub fn asin(&self) -> Self {
        Self {
            expr: self.expr.clone().asin(),
        }
    }

    /// Returns a new parameter representing the cosine `cos(x)`.
    pub fn cos(&self) -> Self {
        Self {
            expr: self.expr.clone().cos(),
        }
    }

    /// Returns a new parameter representing the inverse cosine `acos(x)`.
    pub fn acos(&self) -> Self {
        Self {
            expr: self.expr.clone().acos(),
        }
    }

    /// Returns a new parameter representing the tangent `tan(x)`.
    pub fn tan(&self) -> Self {
        Self {
            expr: self.expr.clone().tan(),
        }
    }

    /// Returns a new parameter representing the inverse tangent `atan(x)`.
    pub fn atan(&self) -> Self {
        Self {
            expr: self.expr.clone().atan(),
        }
    }

    /// Returns a new parameter representing the natural logarithm `ln(x)`.
    pub fn ln(&self) -> Self {
        Self {
            expr: self.expr.clone().ln(),
        }
    }

    /// Returns a new parameter representing the logarithm `log_base(x)`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cqlib_core::circuit::Parameter;
    ///
    /// // log base 2 of x
    /// let x    = Parameter::symbol("x");
    /// let log2 = x.log(Parameter::from(2.0));
    /// let val  = log2.evaluate(&Some([("x", 8.0)].iter().cloned().collect())).unwrap();
    /// assert!((val - 3.0).abs() < 1e-10);
    /// ```
    pub fn log(&self, base: impl Into<Self>) -> Self {
        Self {
            expr: self.expr.clone().log(base.into()),
        }
    }

    /// Returns the mathematical constant π as a symbolic parameter.
    ///
    /// The symbol `"π"` is resolved to [`std::f64::consts::PI`] automatically
    /// inside [`Parameter::evaluate`].
    pub fn pi() -> Self {
        Self::symbol("π")
    }

    /// Returns the mathematical constant *e* (Euler's number) as a symbolic
    /// parameter.
    ///
    /// The symbol `"e"` is resolved to [`std::f64::consts::E`] automatically
    /// inside [`Parameter::evaluate`].
    pub fn e() -> Self {
        Self::symbol("e")
    }

    /// Returns a new parameter representing `self` raised to the power `exp`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cqlib_core::circuit::Parameter;
    ///
    /// let x   = Parameter::symbol("x");
    /// let x3  = x.pow(Parameter::from(3.0));   // x³
    /// let val = x3.evaluate(&Some([("x", 2.0)].iter().cloned().collect())).unwrap();
    /// assert!((val - 8.0).abs() < 1e-10);
    /// ```
    pub fn pow(&self, exp: impl Into<Self>) -> Self {
        Self {
            expr: self.expr.clone().pow(exp.into()),
        }
    }

    /// Substitutes every occurrence of `symbol` in this expression with
    /// `param`, returning the resulting expression.
    /// The inserted expression is not visited again; the original is unchanged.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cqlib_core::circuit::Parameter;
    ///
    /// let expr     = Parameter::symbol("θ") * Parameter::from(2.0);  // θ * 2
    /// let replaced = expr.replace("θ", Parameter::pi());      // π * 2
    /// let val      = replaced.evaluate(&None).unwrap();
    /// assert!((val - 2.0 * std::f64::consts::PI).abs() < 1e-10);
    /// ```
    pub fn replace(&self, symbol: &str, param: impl Into<Self>) -> Self {
        let p: Self = param.into();
        Self {
            expr: substitute_parameter_expr(&self.expr, &|name| {
                (name == symbol).then_some(&p.expr)
            }),
        }
    }

    /// Returns the symbol name if this parameter is exactly one free symbol.
    ///
    /// This is intentionally conservative: expressions containing one symbol
    /// are accepted only when they are structurally equal to that symbol after
    /// optional simplification.
    pub fn as_symbol(&self) -> Option<String> {
        let symbols = self.get_symbols();
        if symbols.len() != 1 {
            return None;
        }

        let symbol = symbols.into_iter().next()?;
        let direct = Self::symbol(&symbol);
        if self == &direct {
            return Some(symbol);
        }

        self.simplify()
            .ok()
            .filter(|simplified| simplified == &direct)
            .map(|_| symbol)
    }

    /// Simultaneously substitutes bound symbols and simplifies when possible.
    ///
    /// Replacement expressions are inserted as-is, without applying other
    /// bindings to them. The result is independent of binding iteration order,
    /// and the original expression is unchanged.
    pub fn substitute_many(&self, bindings: &HashMap<String, Parameter>) -> Self {
        let substituted = self.substitute_many_simultaneous(bindings);
        substituted.simplify().unwrap_or(substituted)
    }

    /// Simultaneously substitutes symbols in a single expression-tree pass.
    ///
    /// Replacement expressions are inserted as-is and are not visited again,
    /// so overlapping bindings such as `a -> b` and `b -> c` do not cascade.
    pub(crate) fn substitute_many_simultaneous(
        &self,
        bindings: &HashMap<String, Parameter>,
    ) -> Self {
        if bindings.is_empty() {
            return self.clone();
        }
        Self::new(substitute_parameter_expr(&self.expr, &|name| {
            bindings.get(name).map(|replacement| &replacement.expr)
        }))
    }

    /// Returns whether two parameters are provably equal within `tolerance`.
    ///
    /// This is a conservative symbolic check.  It accepts structural equality
    /// after simplification and concrete numeric equality, but it does not try
    /// to prove arbitrary symbolic identities.
    pub fn provably_equal(&self, other: &Self, tolerance: f64) -> bool {
        if self == other {
            return true;
        }
        if let (Ok(lhs), Ok(rhs)) = (self.evaluate(&None), other.evaluate(&None))
            && (lhs - rhs).abs() <= tolerance
        {
            return true;
        }

        let lhs = self.simplify().unwrap_or_else(|_| self.clone());
        let rhs = other.simplify().unwrap_or_else(|_| other.clone());
        if lhs == rhs {
            return true;
        }

        match (lhs.evaluate(&None), rhs.evaluate(&None)) {
            (Ok(lhs), Ok(rhs)) => (lhs - rhs).abs() <= tolerance,
            _ => false,
        }
    }

    /// Returns whether two parameters are provably equal modulo `modulus`.
    ///
    /// Fully numeric parameters take a fast path with plain floating-point
    /// math, without building or simplifying a symbolic difference expression.
    /// Only expressions with unbound symbols fall back to the symbolic path,
    /// where the expression difference and modulus must both evaluate without
    /// extra bindings for the numeric modulo check to apply.
    pub fn provably_equal_modulo(&self, other: &Self, modulus: &Self, tolerance: f64) -> bool {
        if let (Some(lhs), Some(rhs)) = (self.numeric_value(), other.numeric_value()) {
            let diff_value = lhs - rhs;
            if diff_value.abs() <= tolerance {
                return true;
            }
            let Some(modulus_value) = modulus.numeric_value() else {
                return false;
            };
            if modulus_value.abs() <= tolerance {
                return false;
            }
            let ratio = diff_value / modulus_value;
            return (ratio - ratio.round()).abs() <= tolerance;
        }

        let raw_diff = self.clone() - other.clone();
        let diff = raw_diff.simplify().unwrap_or(raw_diff);
        if diff.provably_equal(&Self::from(0.0), tolerance) {
            return true;
        }

        let Ok(diff_value) = diff.evaluate(&None) else {
            return false;
        };
        let Ok(modulus_value) = modulus.evaluate(&None) else {
            return false;
        };
        if modulus_value.abs() <= tolerance {
            return false;
        }

        let ratio = diff_value / modulus_value;
        (ratio - ratio.round()).abs() <= tolerance
    }

    /// Returns `true` if this parameter is a constant (has no free variables).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cqlib_core::circuit::Parameter;
    ///
    /// let constant = Parameter::from(3.14);
    /// assert!(constant.is_constant());
    ///
    /// let symbolic = Parameter::symbol("x");
    /// assert!(!symbolic.is_constant());
    /// ```
    pub fn is_constant(&self) -> bool {
        self.get_symbols().is_empty()
    }

    /// Returns `true` if this parameter evaluates to zero.
    ///
    /// Returns `false` if the parameter cannot be evaluated (contains unbound symbols).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cqlib_core::circuit::Parameter;
    ///
    /// let zero = Parameter::from(0.0);
    /// assert!(zero.is_zero());
    ///
    /// let non_zero = Parameter::from(1.0);
    /// assert!(!non_zero.is_zero());
    /// ```
    pub fn is_zero(&self) -> bool {
        self.evaluate(&None)
            .map(|v| v.abs() < f64::EPSILON)
            .unwrap_or(false)
    }

    /// Returns `true` if this parameter evaluates to one.
    ///
    /// Returns `false` if the parameter cannot be evaluated (contains unbound symbols).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cqlib_core::circuit::Parameter;
    ///
    /// let one = Parameter::from(1.0);
    /// assert!(one.is_one());
    ///
    /// let non_one = Parameter::from(2.0);
    /// assert!(!non_one.is_one());
    /// ```
    pub fn is_one(&self) -> bool {
        self.evaluate(&None)
            .map(|v| (v - 1.0).abs() < f64::EPSILON)
            .unwrap_or(false)
    }

    /// Returns the hyperbolic sine `sinh(x)`.
    pub fn sinh(&self) -> Self {
        Self {
            expr: self.expr.clone().sinh(),
        }
    }

    /// Returns the hyperbolic cosine `cosh(x)`.
    pub fn cosh(&self) -> Self {
        Self {
            expr: self.expr.clone().cosh(),
        }
    }

    /// Returns the hyperbolic tangent `tanh(x)`.
    pub fn tanh(&self) -> Self {
        Self {
            expr: self.expr.clone().tanh(),
        }
    }

    /// Returns the floor of the expression.
    pub fn floor(&self) -> Self {
        Self {
            expr: self.expr.clone().floor(),
        }
    }

    /// Returns the ceiling of the expression.
    pub fn ceil(&self) -> Self {
        Self {
            expr: self.expr.clone().ceil(),
        }
    }

    /// Returns the rounded value of the expression.
    pub fn round(&self) -> Self {
        Self {
            expr: self.expr.clone().round(),
        }
    }
}

impl From<f64> for Parameter {
    fn from(value: f64) -> Self {
        if !value.is_finite() {
            panic!("Parameter numeric literal must be finite, got {value}");
        }
        Self {
            expr: Expr::from(value),
        }
    }
}

impl From<f32> for Parameter {
    fn from(value: f32) -> Self {
        if !value.is_finite() {
            panic!("Parameter numeric literal must be finite, got {value}");
        }
        Self {
            expr: Expr::from(f64::from(value)),
        }
    }
}

impl From<u32> for Parameter {
    fn from(value: u32) -> Self {
        Self {
            expr: Expr::from(f64::from(value)),
        }
    }
}

impl From<i32> for Parameter {
    fn from(value: i32) -> Self {
        Self {
            expr: Expr::from(value),
        }
    }
}

/// Implements the four arithmetic operators between `Parameter` and a primitive numeric
/// type `$t` in both directions (e.g. `Parameter + f64` and `f64 + Parameter`).
///
/// The implementation converts `$t` to `Parameter` via `Parameter::from`, extracts the
/// underlying `Expr`, and preserves affine coefficients before falling back to
/// the corresponding symbolic engine operator.
macro_rules! impl_ops_for_type {
    ($t:ty) => {
        // Parameter + T
        impl Add<$t> for Parameter {
            type Output = Parameter;
            fn add(self, rhs: $t) -> Self::Output {
                Self {
                    // 1. `Parameter::from(rhs)` converts the primitive to `Parameter`.
                    // 2. `.expr` unwraps the underlying `Expr` node.
                    // 3. Preserve affine coefficients before general symbolic addition.
                    expr: add_parameter_expr(self.expr, Parameter::from(rhs).expr),
                }
            }
        }

        // T + Parameter
        impl Add<Parameter> for $t {
            type Output = Parameter;
            fn add(self, rhs: Parameter) -> Self::Output {
                Parameter {
                    expr: add_parameter_expr(Parameter::from(self).expr, rhs.expr),
                }
            }
        }

        // Parameter - T
        impl Sub<$t> for Parameter {
            type Output = Parameter;
            fn sub(self, rhs: $t) -> Self::Output {
                Self {
                    expr: sub_parameter_expr(self.expr, Parameter::from(rhs).expr),
                }
            }
        }

        // T - Parameter
        impl Sub<Parameter> for $t {
            type Output = Parameter;
            fn sub(self, rhs: Parameter) -> Self::Output {
                Parameter {
                    expr: sub_parameter_expr(Parameter::from(self).expr, rhs.expr),
                }
            }
        }

        // Parameter * T
        impl Mul<$t> for Parameter {
            type Output = Parameter;
            fn mul(self, rhs: $t) -> Self::Output {
                Self {
                    expr: mul_parameter_expr(self.expr, Parameter::from(rhs).expr),
                }
            }
        }

        // T * Parameter
        impl Mul<Parameter> for $t {
            type Output = Parameter;
            fn mul(self, rhs: Parameter) -> Self::Output {
                Parameter {
                    expr: mul_parameter_expr(Parameter::from(self).expr, rhs.expr),
                }
            }
        }

        // Parameter / T
        impl Div<$t> for Parameter {
            type Output = Parameter;
            fn div(self, rhs: $t) -> Self::Output {
                Self {
                    expr: div_parameter_expr(self.expr, Parameter::from(rhs).expr),
                }
            }
        }

        // T / Parameter
        impl Div<Parameter> for $t {
            type Output = Parameter;
            fn div(self, rhs: Parameter) -> Self::Output {
                Parameter {
                    expr: div_parameter_expr(Parameter::from(self).expr, rhs.expr),
                }
            }
        }
    };
}

// Apply the operator impls to all supported primitive types.
impl_ops_for_type!(f64);
impl_ops_for_type!(f32);
impl_ops_for_type!(i32);
impl_ops_for_type!(u32);

/// Implements all four combinations of owned/borrowed operands for a binary
/// operator between two `Parameter` values.
///
/// For each `(Trait, method)` pair this macro generates:
///
/// 1. `Parameter  op Parameter`  — both operands consumed.
/// 2. `&Parameter op &Parameter` — neither operand consumed (most common in loop contexts).
/// 3. `Parameter  op &Parameter` — left consumed, right borrowed.
/// 4. `&Parameter op Parameter`  — left borrowed, right consumed.
///
/// Cloning `Expr` is inexpensive because `symb_anafis` uses reference-counted
/// node sharing internally.
macro_rules! impl_binary_op_ref {
    ($($trait:ident, $method:ident, $expr_method:ident);* $(;)?) => {
        $(
            // 1. Parameter op Parameter — both sides consumed.
            impl $trait<Parameter> for Parameter {
                type Output = Parameter;
                fn $method(self, rhs: Parameter) -> Self::Output {
                    Parameter {
                        // Preserve affine coefficients before general symbolic arithmetic.
                        expr: $expr_method(self.expr, rhs.expr),
                    }
                }
            }

            // 2. &Parameter op &Parameter — no ownership transferred; clone the inner `Expr`.
            impl<'a, 'b> $trait<&'b Parameter> for &'a Parameter {
                type Output = Parameter;
                fn $method(self, rhs: &'b Parameter) -> Self::Output {
                    Parameter {
                        // `Expr::clone` is cheap — the tree uses Arc-based
                        // node sharing inside `symb_anafis`.
                        expr: $expr_method(self.expr.clone(), rhs.expr.clone()),
                    }
                }
            }

            // 3. Parameter op &Parameter — left consumed, right cloned.
            impl<'a> $trait<&'a Parameter> for Parameter {
                type Output = Parameter;
                fn $method(self, rhs: &'a Parameter) -> Self::Output {
                    Parameter {
                        expr: $expr_method(self.expr, rhs.expr.clone()),
                    }
                }
            }

            // 4. &Parameter op Parameter — left cloned, right consumed.
            impl<'a> $trait<Parameter> for &'a Parameter {
                type Output = Parameter;
                fn $method(self, rhs: Parameter) -> Self::Output {
                    Parameter {
                        expr: $expr_method(self.expr.clone(), rhs.expr),
                    }
                }
            }
        )*
    };
}

impl_binary_op_ref! {
    Add, add, add_parameter_expr;
    Sub, sub, sub_parameter_expr;
    Mul, mul, mul_parameter_expr;
    Div, div, div_parameter_expr;
}

// Implement Neg for owned Parameter
impl Neg for Parameter {
    type Output = Parameter;
    fn neg(self) -> Self::Output {
        Parameter {
            expr: mul_parameter_expr(Expr::number(-1.0), self.expr),
        }
    }
}

// Implement Neg for borrowed Parameter
impl Neg for &Parameter {
    type Output = Parameter;
    fn neg(self) -> Self::Output {
        Parameter {
            expr: mul_parameter_expr(Expr::number(-1.0), self.expr.clone()),
        }
    }
}

#[cfg(test)]
#[path = "parameter_test.rs"]
mod parameter_test;
