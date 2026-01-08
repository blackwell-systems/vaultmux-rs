#!/bin/bash
#
# Run AWS integration tests with LocalStack
#
# Usage: ./scripts/test-aws-localstack.sh

set -e

echo "=== AWS Integration Tests with LocalStack ==="
echo ""

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "Error: Docker is not running"
    echo "Please start Docker and try again"
    exit 1
fi

# Start LocalStack
echo "1. Starting LocalStack..."
CONTAINER_ID=$(docker run -d \
    -p 4566:4566 \
    -e SERVICES=secretsmanager \
    localstack/localstack:latest)

echo "   Container ID: $CONTAINER_ID"

# Wait for LocalStack to be ready
echo ""
echo "2. Waiting for LocalStack to be ready..."
timeout 60 bash -c '
until curl -f http://localhost:4566/_localstack/health 2>/dev/null | grep -q "secretsmanager.*available"; do
    echo "   Waiting..."
    sleep 2
done
' || {
    echo "Error: LocalStack failed to start"
    docker logs "$CONTAINER_ID"
    docker stop "$CONTAINER_ID"
    docker rm "$CONTAINER_ID"
    exit 1
}

echo "   ✓ LocalStack is ready"

# Run tests
echo ""
echo "3. Running AWS integration tests..."
export LOCALSTACK_ENDPOINT=http://localhost:4566
export AWS_ACCESS_KEY_ID=test
export AWS_SECRET_ACCESS_KEY=test
export AWS_REGION=us-east-1

cargo test --test integration_aws --features aws -- --ignored --nocapture || TEST_RESULT=$?

# Clean up
echo ""
echo "4. Cleaning up..."
docker stop "$CONTAINER_ID" > /dev/null
docker rm "$CONTAINER_ID" > /dev/null
echo "   ✓ LocalStack stopped and removed"

# Exit with test result
if [ -n "$TEST_RESULT" ]; then
    echo ""
    echo "✗ Tests failed with exit code $TEST_RESULT"
    exit "$TEST_RESULT"
else
    echo ""
    echo "✓ All tests passed!"
    exit 0
fi
