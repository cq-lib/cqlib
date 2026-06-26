## Description / 变更说明

<!-- Describe what this PR does and why. / 说明这个 PR 做了什么变更以及为什么。 -->

## Related Issues / 关联 Issue

<!-- Link related issues: Closes #123, Related to #456 / 关联的 Issue 编号 -->

## Changes / 具体改动

<!-- List key changes / 列出主要改动点 -->
-

## Component Impact / 影响范围

<!-- Check affected components / 勾选涉及的组件 -->
- [ ] `cqlib-core` (Rust core / 核心库)
- [ ] `binding-python` (Python bindings / Python 绑定)
- [ ] `binding-c` (C bindings / C 绑定)
- [ ] `cqlib` (public Rust crate / 对外 crate)
- [ ] Documentation / 文档
- [ ] CI / Build / 构建系统

## Testing / 测试

<!-- Describe how you tested your changes / 说明你是如何测试这些改动的 -->

### Commands run / 运行的测试命令

```bash
# Rust tests
cargo test --all
cargo test -p cqlib-core
cargo test -p binding-c

# Python tests
maturin develop -m crates/binding-python/Cargo.toml
pytest tests/python/
```

### Test results / 测试结果

<!-- Paste test output or describe results / 贴出测试结果或描述大致情况 -->

## Checklist / 检查清单

- [ ] My change is based on the latest `main` branch / 改动基于最新 `main` 分支
- [ ] I have read the [Contributing Guide](CONTRIBUTING.md) / 已阅读贡献指南
- [ ] Code style checks pass (`cargo fmt`, `cargo clippy`, `ruff check`) / 代码风格检查通过
- [ ] I have added tests that prove my fix/feature works / 已添加测试验证
- [ ] New and existing tests pass locally / 新旧测试全部通过
- [ ] I have updated documentation as needed / 已更新相关文档
- [ ] No unrelated files, debug code, or temporary output / 无无关文件或调试代码
- [ ] Commit messages follow [conventional format](https://www.conventionalcommits.org/) / 提交信息遵循规范格式

## Notes for Reviewers / 评审提示

<!-- Anything reviewers should pay attention to / 需要评审者关注的点 -->
