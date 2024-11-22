DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
. "$DIR/common.sh"

build_and_package x86_64-unknown-linux-gnu ",+avx2" "avx2"