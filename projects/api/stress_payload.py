#!/usr/bin/env python3
import json
import os
import subprocess
import sys
import time
import urllib.request
import urllib.error

PORT = 8086
BASE_URL = f"http://localhost:{PORT}"

def test_payload(label, username, name, bio, expected_status=201):
    url = f"{BASE_URL}/api/users"
    payload = json.dumps({"username": username, "name": name, "bio": bio}).encode("utf-8")
    req = urllib.request.Request(url, data=payload, headers={"Content-Type": "application/json"}, method="POST")

    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            data = resp.read().decode("utf-8")
            status = resp.status
            print(f"[{status}] {label}: Succeeded")
            return status, data
    except urllib.error.HTTPError as e:
        body = e.read().decode("utf-8", errors="replace")
        print(f"[{e.code}] {label}: HTTP Error: {body[:150]}")
        return e.code, body
    except Exception as e:
        print(f"[FAIL] {label}: Exception: {e}")
        return 0, str(e)

def main():
    subprocess.run(["psql", "-d", "rey_social_db", "-f", "schema.sql"], capture_output=True, check=True)

    env = os.environ.copy()
    env["PORT"] = str(PORT)
    server_proc = subprocess.Popen(["node", "server.js"], env=env)

    print(f"Waiting for server on port {PORT}...")
    for _ in range(30):
        try:
            with urllib.request.urlopen(f"{BASE_URL}/api/health", timeout=1) as resp:
                if resp.status == 200:
                    break
        except Exception:
            time.sleep(0.2)

    try:
        print("\n==============================================")
        print(" FIELD CONSTRAINT & PAYLOAD STRESS TESTING")
        print("==============================================")

        # 1. Username boundary: 50 characters (exact limit)
        test_payload("Username: exactly 50 chars", "u" * 50, "Valid Name", "Bio text")

        # 2. Username boundary: 51 characters (exceeds VARCHAR(50))
        test_payload("Username: 51 chars (exceeds VARCHAR(50))", "u" * 51, "Valid Name", "Bio text")

        # 3. Username boundary: 500 characters
        test_payload("Username: 500 chars", "u" * 500, "Valid Name", "Bio text")

        # 4. Bio payload: 10 KB
        test_payload("Bio payload: 10 KB text", "user_10k", "Tester", "A" * 10_000)

        # 5. Bio payload: 100 KB
        test_payload("Bio payload: 100 KB text", "user_100k", "Tester", "B" * 100_000)

        # 6. Bio payload: 1 MB
        test_payload("Bio payload: 1 MB text", "user_1m", "Tester", "C" * 1_000_000)

        # 7. SQL Injection stress test
        test_payload("SQL Injection vector: single quotes", "user_quote", "O'Connor", "Hello'; DROP TABLE users; --")

        # 8. Unicode & Emoji stress test
        test_payload("Unicode & Multi-byte Emojis", "user_emoji_🚀🔥", "Rey Developer ⚡️", "💻 Writing pure Rey & PostgreSQL! 🚀 🦀 🦄")

        # 9. Empty payload
        test_payload("Empty fields validation", "", "", "")

        # 10. Massive CLI argv stress (5 MB bio)
        test_payload("Bio payload: 5 MB text (CLI argv stress)", "user_5m", "Tester", "D" * 5_000_000)

    finally:
        server_proc.terminate()
        server_proc.wait()

if __name__ == "__main__":
    main()
