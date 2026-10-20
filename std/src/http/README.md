# std::http

Small HTTP client helpers for JSON and REST APIs. Requests run `curl` through
`std::process`, so `curl` must be installed and available on `PATH`.

## API

- `request(method: string, url: string, headers: HashMap<string, string>, body: string): any`
- `get(url: string): any`
- `post(url: string, body: string): any`
- `put(url: string, body: string): any`
- `del(url: string): any` (sends the HTTP `DELETE` method)

`request` accepts explicit string headers and a string body. It returns
`Result.Ok({status: int, ok: bool, body: string, stderr: string})` after a
successful `curl` invocation. `ok` is true for 2xx status codes. Process
failures return `Result.Err(message)`. The response body is raw text; JSON
parsing is not included.

The convenience helpers send no headers. Use `request` when custom headers or
another HTTP method are needed. Values passed into a request are shell-quoted
before being added to the `curl` command.
