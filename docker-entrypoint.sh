#!/bin/sh
set -e

# allow the container to be started with `--user`
if [ "$1" = 'api_service' ] && [ "$(id -u)" = '0' ]; then
	exec gosu api-service "$0" "$@"
fi

exec "$@"
