# 认证与配置

使用天衍平台前，需要先完成认证。`cqlib-tianyan` 使用 API Key 登录，登录成功后会创建 `TianyanPlatform` 对象。后续的后端查询、任务提交和结果查询都从这个对象开始。

对应导入：

```python
from cqlib_tianyan import TianyanPlatform, TianyanConfig, TianyanError
```

## 1. 使用 API Key 登录

推荐把 API Key 放在环境变量中，避免写入代码：

```bash
export TIANYAN_API_KEY="your_api_key"
```

Python 示例：

```python
import os
from cqlib_tianyan import TianyanPlatform

platform = TianyanPlatform.login(os.environ["TIANYAN_API_KEY"])
```

默认平台域名是：

```text
qc.zdxlz.com
```

默认 base URL 为：

```text
https://qc.zdxlz.com
```

## 2. 凭据保存与复用

默认情况下，登录成功后会把凭据保存到本地：

| 系统 | 默认路径 |
|---|---|
| macOS / Linux | `~/.cqlib/tianyan/credentials.json` |
| Windows | `%APPDATA%\cqlib\tianyan\credentials.json` |

后续可以直接复用保存的凭据：

```python
from cqlib_tianyan import TianyanPlatform

platform = TianyanPlatform.from_credentials()
```

如果保存的 token 已过期，默认会使用保存的 API Key 自动刷新。

## 3. 不保存凭据

如果不希望把凭据写入磁盘，可以关闭 `save_credentials`：

```python
platform = TianyanPlatform.login(
    os.environ["TIANYAN_API_KEY"],
    save_credentials=False,
)
```

如果希望 token 过期时不要自动刷新，也可以关闭 `auto_refresh`：

```python
platform = TianyanPlatform.login(
    os.environ["TIANYAN_API_KEY"],
    save_credentials=False,
    auto_refresh=False,
)
```

## 4. 自定义凭据路径

```python
platform = TianyanPlatform.login(
    os.environ["TIANYAN_API_KEY"],
    credentials_path="/secure/path/tianyan_credentials.json",
)
```

从自定义路径加载：

```python
platform = TianyanPlatform.from_credentials(
    credentials_path="/secure/path/tianyan_credentials.json",
)
```

## 5. 自定义平台域名

默认情况下不需要设置 `domain`。如果部署环境使用自定义域名，可以这样写：

```python
platform = TianyanPlatform.login(
    os.environ["TIANYAN_API_KEY"],
    domain="qc.zdxlz.com",
)
```

`domain` 只需要传主机名，不需要写 `https://`。

## 6. TianyanConfig

`TianyanConfig` 用于查看或组织配置项。Python 绑定中，`login` 和 `from_credentials` 已经可以直接接收配置关键字参数，因此普通用户不一定需要显式创建 `TianyanConfig`。

```python
from cqlib_tianyan import TianyanConfig

config = TianyanConfig(
    domain="qc.zdxlz.com",
    save_credentials=True,
    auto_refresh=True,
    credentials_path="/secure/path/tianyan_credentials.json",
)

print(config.base_url)
print(config.credentials_path)
```

配置项说明：

| 参数 | 默认值 | 说明 |
|---|---|---|
| `domain` | `qc.zdxlz.com` | 平台主机名 |
| `save_credentials` | `True` | 登录或刷新后是否保存凭据 |
| `auto_refresh` | `True` | token 过期后是否自动重新登录 |
| `credentials_path` | 平台默认路径 | 本地凭据 JSON 文件路径 |

## 7. 错误处理

平台认证、网络请求、任务提交和结果查询失败时会抛出异常。推荐写法：

```python
from cqlib_tianyan import TianyanPlatform, TianyanError

try:
    platform = TianyanPlatform.login("invalid_api_key")
except Exception as exc:
    print(f"登录失败: {exc}")
```

当前 Python abi3 绑定中，实际使用时建议捕获 `Exception`，再根据错误信息进行处理。

## 8. 安全建议

- 不要把 API Key 写进源码仓库。
- 优先使用环境变量或密钥管理系统保存 API Key。
- 多人共享机器上建议设置自定义 `credentials_path`。
- CI 中建议关闭凭据持久化：`save_credentials=False`。
- 如果只做临时测试，可以同时关闭 `save_credentials` 和 `auto_refresh`。

## 下一步

- [后端与设备配置](2_backend_device.md)：列举平台后端、选择目标设备，并获取设备拓扑和校准信息。
- [任务提交与结果获取](3_task_result.md)：在完成后端选择后，提交 QCIS 线路并轮询云端执行结果。
