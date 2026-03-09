# set -e

cargo test --quiet --profile test > /tmp/onoro_output.txt
if [ $? -ne 0 ]; then
  cat /tmp/onoro_output.txt
  exit -1
fi

cd onoro_impl && cargo test --quiet --profile test > /tmp/onoro_output.txt
if [ $? -ne 0 ]; then
  cat /tmp/onoro_output.txt
  exit -1
fi

echo "All tests pass"
