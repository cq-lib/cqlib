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

//! Python binding for symbolic and numeric circuit parameters.
//!
//! The wrapper follows the core [`Parameter`] expression model: strings are
//! parsed as expressions, numeric values must be finite, arithmetic builds new
//! immutable expressions, and evaluation errors are exposed as the dedicated
//! Python `ParameterError` type.

use crate::circuit::error::ParameterError as PyParameterError;
use crate::utils::hash_value;
use cqlib_core::circuit::Parameter;
use cqlib_core::circuit::error::ParameterError;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use std::collections::HashMap;

/// Immutable symbolic or numeric expression used as a circuit parameter.
#[pyclass(name = "Parameter", module = "cqlib.circuit", from_py_object)]
#[derive(Clone, Debug)]
pub struct PyParameter {
    pub(crate) inner: Parameter,
}

#[pymethods]
impl PyParameter {
    /// Creates a parameter from a finite number or expression string.
    ///
    /// A plain identifier such as `"theta"` is parsed as a symbol. Invalid
    /// expression syntax raises `ParameterError` instead of silently creating a
    /// symbol with the invalid text.
    #[new]
    fn new(value: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(number) = value.extract::<f64>() {
            if !number.is_finite() {
                return Err(PyParameterError::new_err(
                    ParameterError::DomainError(format!(
                        "numeric parameter must be finite, got {number}"
                    ))
                    .to_string(),
                ));
            }
            return Ok(Self {
                inner: Parameter::from(number),
            });
        }

        if let Ok(expression) = value.extract::<String>() {
            return Parameter::try_from(expression.as_str())
                .map(|inner| Self { inner })
                .map_err(|error| PyParameterError::new_err(error.to_string()));
        }

        Err(PyTypeError::new_err(
            "Parameter value must be a number or string",
        ))
    }

    /// Parses a mathematical expression string into a Parameter.
    ///
    /// # Arguments
    ///
    /// * `expr` - The expression string to parse (e.g., "theta + 1", "pi/2", "sin(x)").
    ///
    /// # Returns
    ///
    /// A new Parameter representing the parsed expression.
    ///
    /// # Supported Syntax
    ///
    /// - Numbers: `1`, `3.14`, `-2.5`
    /// - Constants: `pi`, `e`
    /// - Variables: `theta`, `x`, `y`
    /// - Operators: `+`, `-`, `*`, `/`
    /// - Functions: `sin`, `cos`, `tan`, `exp`, `sqrt`, `ln`, etc.
    /// - Parentheses: `(`, `)`
    #[staticmethod]
    fn from_expression(expr: String) -> PyResult<Self> {
        Parameter::try_from(expr.as_str())
            .map(|inner| Self { inner })
            .map_err(|error| PyParameterError::new_err(error.to_string()))
    }

    /// Returns the mathematical constant Pi.
    #[staticmethod]
    fn pi() -> Self {
        PyParameter {
            inner: Parameter::pi(),
        }
    }

    /// Returns the mathematical constant e (Euler's number).
    #[staticmethod]
    fn e() -> Self {
        PyParameter {
            inner: Parameter::e(),
        }
    }

    /// Evaluates the parameter with concrete values.
    ///
    /// # Arguments
    ///
    /// * `bindings` - Optional mapping from symbol names to values.
    ///
    /// # Returns
    ///
    /// The computed floating-point result.
    #[pyo3(signature = (bindings=None))]
    fn evaluate(&self, bindings: Option<HashMap<String, f64>>) -> PyResult<f64> {
        let bindings_ref = bindings.as_ref().map(|map| {
            map.iter()
                .map(|(k, v)| (k.as_str(), *v))
                .collect::<HashMap<&str, f64>>()
        });
        self.inner
            .evaluate(&bindings_ref)
            .map_err(|error| PyParameterError::new_err(error.to_string()))
    }

    /// Simplifies the parameter expression.
    ///
    /// Returns a domain-safe algebraically simplified expression.
    #[pyo3(signature = ())]
    fn simplify(&self) -> PyResult<Self> {
        let inner = self
            .inner
            .simplify()
            .map_err(|error| PyParameterError::new_err(error.to_string()))?;
        Ok(Self { inner })
    }

    /// Computes the symbolic derivative with respect to a variable.
    ///
    /// # Arguments
    ///
    /// * `var` - The variable name to differentiate by.
    ///
    /// # Returns
    ///
    /// A new parameter representing the derivative.
    fn derivative(&self, var: String) -> PyResult<Self> {
        let inner = self
            .inner
            .derivative(&var)
            .map_err(|error| PyParameterError::new_err(error.to_string()))?;
        Ok(Self { inner })
    }

    /// Returns the power of this parameter raised to the given exponent.
    ///
    /// # Arguments
    ///
    /// * `val` - The exponent (can be a float or Parameter).
    ///
    /// # Returns
    ///
    /// A new parameter representing `self^val`.
    ///
    /// # Example
    ///
    /// ```python
    /// x = Parameter("x")
    /// y = Parameter("y")
    /// result = x.pow(y)  # x^y
    /// result = x.pow(2)  # x^2
    /// ```
    fn pow(&self, val: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(param) = val.extract::<PyParameter>() {
            Ok(PyParameter {
                inner: self.inner.pow(param.inner),
            })
        } else if let Ok(val_f64) = val.extract::<f64>() {
            if !val_f64.is_finite() {
                return Err(PyParameterError::new_err(
                    ParameterError::DomainError(format!(
                        "numeric parameter must be finite, got {val_f64}"
                    ))
                    .to_string(),
                ));
            }
            let exp_param = Parameter::from(val_f64);
            Ok(PyParameter {
                inner: self.inner.pow(exp_param),
            })
        } else {
            Err(PyTypeError::new_err(
                "pow argument must be a number or Parameter",
            ))
        }
    }

    /// Returns all unique symbols in this parameter.
    #[getter]
    fn symbols(&self) -> Vec<String> {
        let mut symbols: Vec<_> = self.inner.get_symbols().into_iter().collect();
        symbols.sort();
        symbols
    }

    /// Returns the canonical storage form used by circuit parameter interning.
    fn canonicalized(&self) -> PyResult<Self> {
        self.inner
            .canonicalized()
            .map(|inner| Self { inner })
            .map_err(|error| PyParameterError::new_err(error.to_string()))
    }

    /// Returns whether this expression is exactly the numeric constant zero.
    fn is_exact_zero(&self) -> PyResult<bool> {
        self.inner
            .is_exact_zero()
            .map_err(|error| PyParameterError::new_err(error.to_string()))
    }

    /// Returns the symbol name when this expression is exactly one symbol.
    fn as_symbol(&self) -> Option<String> {
        self.inner.as_symbol()
    }

    /// Substitutes multiple symbols and simplifies the resulting expression.
    fn substitute(&self, bindings: HashMap<String, PyParameter>) -> Self {
        let bindings = bindings
            .into_iter()
            .map(|(symbol, parameter)| (symbol, parameter.inner))
            .collect();
        Self {
            inner: self.inner.substitute_many(&bindings),
        }
    }

    /// Conservatively checks equality within a numeric tolerance.
    #[pyo3(signature = (other, tolerance=1e-12))]
    fn provably_equal(&self, other: PyParameter, tolerance: f64) -> bool {
        self.inner.provably_equal(&other.inner, tolerance)
    }

    /// Conservatively checks equality modulo `modulus` within a tolerance.
    #[pyo3(signature = (other, modulus, tolerance=1e-12))]
    fn provably_equal_modulo(
        &self,
        other: PyParameter,
        modulus: PyParameter,
        tolerance: f64,
    ) -> bool {
        self.inner
            .provably_equal_modulo(&other.inner, &modulus.inner, tolerance)
    }

    /// Returns the absolute value |x|.
    fn abs(&self) -> Self {
        PyParameter {
            inner: self.inner.abs(),
        }
    }

    /// Returns the square root sqrt(x).
    fn sqrt(&self) -> Self {
        PyParameter {
            inner: self.inner.sqrt(),
        }
    }

    /// Returns the exponential e^x.
    fn exp(&self) -> Self {
        PyParameter {
            inner: self.inner.exp(),
        }
    }

    /// Returns the sine sin(x).
    fn sin(&self) -> Self {
        PyParameter {
            inner: self.inner.sin(),
        }
    }

    /// Returns the cosine cos(x).
    fn cos(&self) -> Self {
        PyParameter {
            inner: self.inner.cos(),
        }
    }

    /// Returns the tangent tan(x).
    fn tan(&self) -> Self {
        PyParameter {
            inner: self.inner.tan(),
        }
    }

    /// Returns the inverse sine asin(x).
    fn asin(&self) -> Self {
        PyParameter {
            inner: self.inner.asin(),
        }
    }

    /// Returns the inverse cosine acos(x).
    fn acos(&self) -> Self {
        PyParameter {
            inner: self.inner.acos(),
        }
    }

    /// Returns the inverse tangent atan(x).
    fn atan(&self) -> Self {
        PyParameter {
            inner: self.inner.atan(),
        }
    }

    /// Returns the natural logarithm ln(x).
    fn ln(&self) -> Self {
        PyParameter {
            inner: self.inner.ln(),
        }
    }

    /// Returns the logarithm. If base is None, returns natural log.
    ///
    /// # Arguments
    ///
    /// * `base` - Optional base for the logarithm.
    #[pyo3(signature = (base=None))]
    fn log(&self, base: Option<PyParameter>) -> Self {
        let base_inner = base.map(|p| p.inner).unwrap_or(Parameter::e());
        PyParameter {
            inner: self.inner.log(base_inner),
        }
    }

    /// Returns the hyperbolic sine sinh(x).
    fn sinh(&self) -> Self {
        PyParameter {
            inner: self.inner.sinh(),
        }
    }

    /// Returns the hyperbolic cosine cosh(x).
    fn cosh(&self) -> Self {
        PyParameter {
            inner: self.inner.cosh(),
        }
    }

    /// Returns the hyperbolic tangent tanh(x).
    fn tanh(&self) -> Self {
        PyParameter {
            inner: self.inner.tanh(),
        }
    }

    /// Returns the floor of the expression.
    fn floor(&self) -> Self {
        PyParameter {
            inner: self.inner.floor(),
        }
    }

    /// Returns the ceiling of the expression.
    fn ceil(&self) -> Self {
        PyParameter {
            inner: self.inner.ceil(),
        }
    }

    /// Returns the rounded value of the expression.
    fn round(&self) -> Self {
        PyParameter {
            inner: self.inner.round(),
        }
    }

    /// Returns True if this parameter is a constant (has no free variables).
    ///
    /// # Examples
    ///
    /// ```python
    /// >>> Parameter(3.14).is_constant()
    /// True
    /// >>> Parameter("x").is_constant()
    /// False
    /// ```
    fn is_constant(&self) -> bool {
        self.inner.is_constant()
    }

    /// Returns True if this parameter evaluates to zero.
    ///
    /// Returns False if the parameter cannot be evaluated (contains unbound symbols).
    ///
    /// # Examples
    ///
    /// ```python
    /// >>> Parameter(0.0).is_zero()
    /// True
    /// >>> Parameter(1.0).is_zero()
    /// False
    /// ```
    fn is_zero(&self) -> bool {
        self.inner.is_zero()
    }

    /// Returns True if this parameter evaluates to one.
    ///
    /// Returns False if the parameter cannot be evaluated (contains unbound symbols).
    ///
    /// # Examples
    ///
    /// ```python
    /// >>> Parameter(1.0).is_one()
    /// True
    /// >>> Parameter(2.0).is_one()
    /// False
    /// ```
    fn is_one(&self) -> bool {
        self.inner.is_one()
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("Parameter({:?})", self.inner.to_string())
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }

    fn __add__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(param) = other.extract::<PyParameter>() {
            Ok(PyParameter {
                inner: self.inner.clone() + param.inner,
            })
        } else if let Ok(val) = other.extract::<f64>() {
            if !val.is_finite() {
                return Err(PyParameterError::new_err(
                    ParameterError::DomainError(format!(
                        "numeric parameter must be finite, got {val}"
                    ))
                    .to_string(),
                ));
            }
            Ok(PyParameter {
                inner: self.inner.clone() + val,
            })
        } else {
            Err(PyTypeError::new_err("Unsupported operand type for +"))
        }
    }

    fn __radd__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        self.__add__(other)
    }

    fn __sub__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(param) = other.extract::<PyParameter>() {
            Ok(PyParameter {
                inner: self.inner.clone() - param.inner,
            })
        } else if let Ok(val) = other.extract::<f64>() {
            if !val.is_finite() {
                return Err(PyParameterError::new_err(
                    ParameterError::DomainError(format!(
                        "numeric parameter must be finite, got {val}"
                    ))
                    .to_string(),
                ));
            }
            Ok(PyParameter {
                inner: self.inner.clone() - val,
            })
        } else {
            Err(PyTypeError::new_err("Unsupported operand type for -"))
        }
    }

    fn __rsub__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(val) = other.extract::<f64>() {
            if !val.is_finite() {
                return Err(PyParameterError::new_err(
                    ParameterError::DomainError(format!(
                        "numeric parameter must be finite, got {val}"
                    ))
                    .to_string(),
                ));
            }
            Ok(PyParameter {
                inner: val - self.inner.clone(),
            })
        } else {
            Err(PyTypeError::new_err("Unsupported operand type for -"))
        }
    }

    fn __mul__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(param) = other.extract::<PyParameter>() {
            Ok(PyParameter {
                inner: self.inner.clone() * param.inner,
            })
        } else if let Ok(val) = other.extract::<f64>() {
            if !val.is_finite() {
                return Err(PyParameterError::new_err(
                    ParameterError::DomainError(format!(
                        "numeric parameter must be finite, got {val}"
                    ))
                    .to_string(),
                ));
            }
            Ok(PyParameter {
                inner: self.inner.clone() * val,
            })
        } else {
            Err(PyTypeError::new_err("Unsupported operand type for *"))
        }
    }

    fn __rmul__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        self.__mul__(other)
    }

    fn __truediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(param) = other.extract::<PyParameter>() {
            Ok(PyParameter {
                inner: self.inner.clone() / param.inner,
            })
        } else if let Ok(val) = other.extract::<f64>() {
            if !val.is_finite() {
                return Err(PyParameterError::new_err(
                    ParameterError::DomainError(format!(
                        "numeric parameter must be finite, got {val}"
                    ))
                    .to_string(),
                ));
            }
            Ok(PyParameter {
                inner: self.inner.clone() / val,
            })
        } else {
            Err(PyTypeError::new_err("Unsupported operand type for /"))
        }
    }

    fn __rtruediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(val) = other.extract::<f64>() {
            if !val.is_finite() {
                return Err(PyParameterError::new_err(
                    ParameterError::DomainError(format!(
                        "numeric parameter must be finite, got {val}"
                    ))
                    .to_string(),
                ));
            }
            Ok(PyParameter {
                inner: val / self.inner.clone(),
            })
        } else {
            Err(PyTypeError::new_err("Unsupported operand type for /"))
        }
    }

    fn __pow__(
        &self,
        other: &Bound<'_, PyAny>,
        modulo: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        if modulo.is_some() {
            return Err(PyTypeError::new_err(
                "Parameter exponentiation does not support a modulo argument",
            ));
        }
        if let Ok(param) = other.extract::<PyParameter>() {
            Ok(PyParameter {
                inner: self.inner.pow(param.inner),
            })
        } else if let Ok(val) = other.extract::<f64>() {
            if !val.is_finite() {
                return Err(PyParameterError::new_err(
                    ParameterError::DomainError(format!(
                        "numeric parameter must be finite, got {val}"
                    ))
                    .to_string(),
                ));
            }
            let exp_param = Parameter::from(val);
            Ok(PyParameter {
                inner: self.inner.pow(exp_param),
            })
        } else {
            Err(PyTypeError::new_err("Unsupported operand type for **"))
        }
    }

    fn __neg__(&self) -> Self {
        PyParameter {
            inner: Parameter::from(0.0) - self.inner.clone(),
        }
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        if let Ok(param) = other.extract::<PyParameter>() {
            self.inner == param.inner
        } else {
            false
        }
    }

    fn __hash__(&self) -> u64 {
        hash_value(&self.inner)
    }

    /// Replaces all occurrences of a symbol with another parameter expression.
    ///
    /// This method performs symbolic substitution, replacing every instance of the
    /// specified symbol with the given parameter's expression tree.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The name of the symbol to replace.
    /// * `param` - The parameter expression to substitute.
    ///
    /// # Returns
    ///
    /// A new `Parameter` with the substitution applied. The original parameter is unchanged.
    ///
    /// # Examples
    ///
    /// ```python
    /// from cqlib import Parameter
    ///
    /// # Create expression: x + 2
    /// x = Parameter("x")
    /// expr = x + Parameter(2.0)
    ///
    /// # Replace x with y * 3
    /// y = Parameter("y")
    /// replacement = y * Parameter(3.0)
    /// new_expr = expr.replace("x", replacement)
    /// # Result: (y * 3) + 2
    /// ```
    fn replace(&self, symbol: String, param: PyParameter) -> Self {
        PyParameter {
            inner: self.inner.replace(&symbol, param.inner),
        }
    }
}

impl PyParameter {
    /// Returns a reference to the inner core Parameter.
    pub fn inner(&self) -> &Parameter {
        &self.inner
    }

    /// Consumes the wrapper and returns the inner core Parameter.
    pub fn into_inner(self) -> Parameter {
        self.inner
    }
}

/// Wraps a core parameter for return values from other binding modules.
impl From<Parameter> for PyParameter {
    fn from(inner: Parameter) -> Self {
        PyParameter { inner }
    }
}

#[cfg(test)]
mod tests {
    use super::PyParameter;
    use crate::circuit::error::ParameterError as PyParameterError;
    use pyo3::IntoPyObjectExt;
    use pyo3::Python;

    #[test]
    fn invalid_expression_uses_parameter_error() {
        Python::initialize();
        Python::attach(|py| {
            let value = "@@@".into_py_any(py).unwrap();
            let error = PyParameter::new(value.bind(py)).unwrap_err();

            assert!(error.is_instance_of::<PyParameterError>(py));
            assert!(error.value(py).to_string().starts_with("Parse error:"));
        });
    }

    #[test]
    fn non_finite_number_uses_parameter_error_without_panicking() {
        Python::initialize();
        Python::attach(|py| {
            let value = f64::NAN.into_py_any(py).unwrap();
            let error = PyParameter::new(value.bind(py)).unwrap_err();

            assert!(error.is_instance_of::<PyParameterError>(py));
            assert!(error.value(py).to_string().contains("must be finite"));
        });
    }

    #[test]
    fn symbols_are_sorted_for_stable_python_results() {
        let parameter = PyParameter {
            inner: cqlib_core::circuit::Parameter::try_from("z + a + m").unwrap(),
        };

        assert_eq!(parameter.symbols(), vec!["a", "m", "z"]);
    }

    #[test]
    fn repr_quotes_the_expression() {
        let parameter = PyParameter {
            inner: cqlib_core::circuit::Parameter::symbol("theta"),
        };

        assert_eq!(parameter.__repr__(), "Parameter(\"theta\")");
        assert_eq!(parameter.as_symbol().as_deref(), Some("theta"));
    }

    #[test]
    fn canonicalized_constant_matches_core_storage_form() {
        let parameter = PyParameter {
            inner: cqlib_core::circuit::Parameter::try_from("1 + 1").unwrap(),
        };
        let canonical = parameter.canonicalized().unwrap();

        assert!(canonical.inner.is_constant());
        assert_eq!(canonical.inner.evaluate(&None).unwrap(), 2.0);
    }
}
