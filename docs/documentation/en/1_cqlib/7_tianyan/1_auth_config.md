# Authentication and configuration

Before using the Tianyan platform, authentication must be completed first. `cqlib-tianyan` logs in with an API Key, and a `TianyanPlatform` object is created after a successful login. Subsequent backend queries, task submission and result queries all start from this object.

Corresponding import:

```python
from cqlib_tianyan import TianyanPlatform, TianyanConfig, TianyanError
```

## 1. Logging in with an API Key

It is recommended to keep the API Key in an environment variable rather than writing it into code:

```bash
export TIANYAN_API_KEY="your_api_key"
```

Python example:

```python
import os
from cqlib_tianyan import TianyanPlatform

platform = TianyanPlatform.login(os.environ["TIANYAN_API_KEY"])
```

The default platform domain is:

```text
qc.zdxlz.com
```

The default base URL is:

```text
https://qc.zdxlz.com
```

## 2. Credential storage and reuse

By default, credentials are saved locally after a successful login:

| System | Default path |
|---|---|
| macOS / Linux | `~/.cqlib/tianyan/credentials.json` |
| Windows | `%APPDATA%\cqlib\tianyan\credentials.json` |

The saved credentials can later be reused directly:

```python
from cqlib_tianyan import TianyanPlatform

platform = TianyanPlatform.from_credentials()
```

If the saved token has expired, the saved API Key is used to refresh it automatically by default.

## 3. Disabling credential storage

To avoid writing credentials to disk, `save_credentials` can be turned off:

```python
platform = TianyanPlatform.login(
    os.environ["TIANYAN_API_KEY"],
    save_credentials=False,
)
```

To avoid automatic refresh when the token expires, `auto_refresh` can also be turned off:

```python
platform = TianyanPlatform.login(
    os.environ["TIANYAN_API_KEY"],
    save_credentials=False,
    auto_refresh=False,
)
```

## 4. Custom credential path

```python
platform = TianyanPlatform.login(
    os.environ["TIANYAN_API_KEY"],
    credentials_path="/secure/path/tianyan_credentials.json",
)
```

Loading from a custom path:

```python
platform = TianyanPlatform.from_credentials(
    credentials_path="/secure/path/tianyan_credentials.json",
)
```

## 5. Custom platform domain

By default, `domain` does not need to be set. If the deployment environment uses a custom domain, write:

```python
platform = TianyanPlatform.login(
    os.environ["TIANYAN_API_KEY"],
    domain="qc.zdxlz.com",
)
```

`domain` only needs the host name; `https://` is not required.

## 6. TianyanConfig

`TianyanConfig` is used to view or organize configuration options. In the Python bindings, `login` and `from_credentials` already accept configuration keyword arguments directly, so ordinary users do not necessarily need to create a `TianyanConfig` explicitly.

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

Configuration options:

| Parameter | Default value | Description |
|---|---|---|
| `domain` | `qc.zdxlz.com` | Platform host name |
| `save_credentials` | `True` | Whether to save credentials after login or refresh |
| `auto_refresh` | `True` | Whether to log in again automatically after the token expires |
| `credentials_path` | Platform default path | Path to the local credentials JSON file |

## 7. Error handling

Exceptions are raised when platform authentication, network requests, task submission or result queries fail. Recommended pattern:

```python
from cqlib_tianyan import TianyanPlatform, TianyanError

try:
    platform = TianyanPlatform.login("invalid_api_key")
except Exception as exc:
    print(f"login failed: {exc}")
```

In the current Python abi3 bindings, it is recommended to catch `Exception` in practice and then handle it according to the error message.

## 8. Security recommendations

- Do not write API Keys into a source code repository.
- Prefer environment variables or a secret management system to store API Keys.
- On a shared machine, a custom `credentials_path` is recommended.
- In CI, disable credential persistence: `save_credentials=False`.
- For temporary testing only, `save_credentials` and `auto_refresh` can be turned off at the same time.

## Next steps

- [Backend and device configuration](2_backend_device.md): list platform backends, select a target device, and obtain device topology and calibration information.
- [Task submission and result retrieval](3_task_result.md): after backend selection is complete, submit QCIS circuits and poll for cloud execution results.
