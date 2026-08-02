# Bundled Python (optional / legacy)

Cluster Runtime **does not ship** a Python interpreter in the installer.

On each machine, at startup it will:

1. Use `CLUSTER_RUNTIME_PYTHON` if set
2. Else use a compatible **system** Python 3.x (packages install into CR-managed venvs)
3. Else reuse `{data_dir}/python/` if previously downloaded
4. Else **download** python-build-standalone (3.10.x) into `{data_dir}/python/`

`scripts/Setup-PythonRuntime.ps1` / `.sh` remain available for **manual** staging
(dev or air-gapped). Populating this `resources/python/` folder is optional and
not required for release builds.
