# std::process

Process and environment helpers for Rey v0.3. Import with
`import std::process;`.

- `args(): Vec<string>` returns command-line arguments.
- `exit(code: int): void` exits the current process.
- `env(key: string): Option<string>` reads an environment variable.
- `setEnv(key: string, value: string): void` and `unsetEnv(key: string): void`
  update the current process environment.
- `envAll(): HashMap<string, string>` returns the environment map.
- `cwd(): string` returns the current working directory.
- `chdir(path: string): void` changes the current working directory.
- `tempDir(): string` returns the system temporary directory.
- `sleepMs(ms: int): void` sleeps for the given number of milliseconds.
- `which(name: string): Option<string>` finds an executable on `PATH`.
- `run(cmd: string): Result<string, string>` executes a shell command and
  returns stdout on success.
- `runDetailed(cmd: string): ExecResult` returns command status, stdout, and
  stderr.
- `isWindows(): bool`, `isMac(): bool`, and `isLinux(): bool` identify the
  current operating system. Windows detection checks `OS=Windows_NT`; macOS
  and Linux detection use `uname -s`.

`run` and `runDetailed` use `sh -c` on Unix-like systems and `cmd /C` on
Windows.

