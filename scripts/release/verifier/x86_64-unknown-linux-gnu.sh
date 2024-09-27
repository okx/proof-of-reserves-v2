DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
. "$DIR/common.sh"

if [ -z "$1" ]; then
    echo "Version Number not Provided. Abort. "
	exit 1
else
    build_and_package x86_64-unknown-linux-gnu "$1"
fi