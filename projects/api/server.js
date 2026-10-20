// HTTP Server Bridge for Rey Social Media API
// Dispatches incoming HTTP requests to Rey CLI backend

const http = require('http');
const { spawn } = require('child_process');
const path = require('path');

const PORT = process.env.PORT || 8080;
const REYC_BIN = process.env.REYC_BIN || '$REPO_ROOT/compiler/v1/target/release/rey-v0';
const MAIN_FILE = path.join(__dirname, 'main.rey');

function callRey(method, reqPath, body) {
  return new Promise((resolve, reject) => {
    const args = ['run', MAIN_FILE, '--', 'handle', method, reqPath];
    if (body) {
      args.push(body);
    }

    const proc = spawn(REYC_BIN, args, { cwd: __dirname });
    let stdout = '';
    let stderr = '';

    proc.stdout.on('data', (d) => { stdout += d.toString(); });
    proc.stderr.on('data', (d) => { stderr += d.toString(); });

    proc.on('close', (code) => {
      if (code !== 0 && !stdout.includes('STATUS:')) {
        return reject(new Error(`Rey failed with code ${code}: ${stderr || stdout}`));
      }

      // Parse STATUS:<status>\nBODY:<body>
      let status = 200;
      let respBody = '{}';

      const statusMatch = stdout.match(/^STATUS:(\d+)/m);
      if (statusMatch) {
        status = parseInt(statusMatch[1], 10);
      }

      const bodyIdx = stdout.indexOf('BODY:');
      if (bodyIdx >= 0) {
        respBody = stdout.slice(bodyIdx + 5).trim();
      } else {
        respBody = stdout.trim();
      }

      resolve({ status, body: respBody });
    });
  });
}

const server = http.createServer(async (req, res) => {
  // CORS Headers
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'GET, POST, PUT, DELETE, OPTIONS');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type, Authorization');

  if (req.method === 'OPTIONS') {
    res.writeHead(204);
    res.end();
    return;
  }

  let body = '';
  req.on('data', (chunk) => { body += chunk.toString(); });
  req.on('end', async () => {
    try {
      const url = new URL(req.url, `http://${req.headers.host || 'localhost'}`);
      const reqPath = url.pathname;

      const result = await callRey(req.method, reqPath, body);

      res.writeHead(result.status, { 'Content-Type': 'application/json' });
      res.end(result.body);
    } catch (err) {
      console.error('Server error:', err);
      res.writeHead(500, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ error: err.message, status: 500 }));
    }
  });
});

server.listen(PORT, () => {
  console.log(`Rey Social Media API running at http://localhost:${PORT}`);
  console.log(`Using compiler binary: ${REYC_BIN}`);
});

process.on('SIGTERM', () => server.close());
process.on('SIGINT', () => server.close());
