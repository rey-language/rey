#!/usr/bin/env bash
set -e

PORT=8082
export PORT
SERVER_PID=""

cleanup() {
    if [ -n "$SERVER_PID" ]; then
        kill "$SERVER_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT

echo "=============================================="
echo " Starting Rey Social Media API HTTP Server on port $PORT"
echo "=============================================="

# Reset schema first
$REPO_ROOT/compiler/v1/target/release/rey-v0 run main.rey -- init-db

node server.js &
SERVER_PID=$!

echo "Waiting for server to become ready..."
for i in {1..20}; do
    if curl -s "http://localhost:$PORT/api/health" > /dev/null; then
        echo "Server is ready!"
        break
    fi
    sleep 0.5
done

echo ""
echo "[Test 1] GET /api/health"
HEALTH=$(curl -s "http://localhost:$PORT/api/health")
echo "Response: $HEALTH"
echo "$HEALTH" | grep -q '"status":"healthy"' || { echo "Health check failed"; exit 1; }

echo ""
echo "[Test 2] POST /api/users (Create User: @octocat)"
USER1=$(curl -s -X POST "http://localhost:$PORT/api/users" -H "Content-Type: application/json" -d '{"username":"octocat","name":"Mona Lisa Octocat","bio":"GitHub Mascot"}')
echo "Response: $USER1"
echo "$USER1" | grep -q '"username":"octocat"' || { echo "User creation failed"; exit 1; }

echo ""
echo "[Test 3] POST /api/users (Create User: @tux)"
USER2=$(curl -s -X POST "http://localhost:$PORT/api/users" -H "Content-Type: application/json" -d '{"username":"tux","name":"Tux Penguin","bio":"Linux Kernel Mascot"}')
echo "Response: $USER2"
echo "$USER2" | grep -q '"username":"tux"' || { echo "User creation failed"; exit 1; }

echo ""
echo "[Test 4] GET /api/users (List Users)"
USERS=$(curl -s "http://localhost:$PORT/api/users")
echo "Response: $USERS"
echo "$USERS" | grep -q 'octocat' || { echo "List users failed"; exit 1; }

echo ""
echo "[Test 5] POST /api/posts (Publish Post from Tux)"
POST1=$(curl -s -X POST "http://localhost:$PORT/api/posts" -H "Content-Type: application/json" -d '{"user_id":2,"content":"Hello from the Linux Kernel on Rey!"}')
echo "Response: $POST1"
echo "$POST1" | grep -q '"content":"Hello from the Linux Kernel on Rey!"' || { echo "Post creation failed"; exit 1; }

echo ""
echo "[Test 6] POST /api/follow (Octocat follows Tux)"
FOLLOW=$(curl -s -X POST "http://localhost:$PORT/api/follow" -H "Content-Type: application/json" -d '{"follower_id":1,"following_id":2}')
echo "Response: $FOLLOW"
echo "$FOLLOW" | grep -q '"message":"Followed successfully"' || { echo "Follow failed"; exit 1; }

echo ""
echo "[Test 7] POST /api/posts/1/like (Octocat likes Tux post)"
LIKE=$(curl -s -X POST "http://localhost:$PORT/api/posts/1/like" -H "Content-Type: application/json" -d '{"user_id":1}')
echo "Response: $LIKE"
echo "$LIKE" | grep -q '"message":"Post liked"' || { echo "Like failed"; exit 1; }

echo ""
echo "[Test 8] GET /api/feed/1 (Fetch Octocat personalized feed)"
FEED=$(curl -s "http://localhost:$PORT/api/feed/1")
echo "Response: $FEED"
echo "$FEED" | grep -q 'Hello from the Linux Kernel on Rey!' || { echo "Feed fetch failed"; exit 1; }

echo ""
echo "[Test 9] GET /api/users/1 (Check Octocat Profile stats: following=1)"
PROFILE=$(curl -s "http://localhost:$PORT/api/users/1")
echo "Response: $PROFILE"
echo "$PROFILE" | grep -q '"following_count":1' || { echo "Profile stats check failed"; exit 1; }

echo ""
echo "[Test 10] POST /api/users (Duplicate user test - expect 409)"
STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "http://localhost:$PORT/api/users" -H "Content-Type: application/json" -d '{"username":"octocat","name":"Duplicate"}')
echo "HTTP Status: $STATUS"
if [ "$STATUS" != "409" ]; then
    echo "Expected status 409, got $STATUS"
    exit 1
fi

echo ""
echo "=============================================="
echo " ALL 10 HTTP INTEGRATION TESTS PASSED CLEANLY!"
echo " Real HTTP Client <-> Node Bridge <-> Rey <-> PostgreSQL"
echo "=============================================="
