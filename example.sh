#!/bin/sh

set -u

wsm="./pgcopy2arrow-ipc.wasm"

f1='id,Int64,false'
f2='timestamp,Int64,false'
f3='severity,Utf8,false'
f4='body,Utf8,false'

fields="${f1};${f2};${f3};${f4}"

run_wasmtime() {
	cat /dev/stdin |
		wasmtime \
			run \
			--env ENV_SPLIT4SCHEMA=';' \
			--env ENV_SPLIT4FIELD=',' \
			--env ENV_SIMPLE_SCHEMA_DEF="${fields}" \
			--env ENV_BAT_SIZE=8192 \
			"${wsm}"
}

run_wasi() {
	run_wasmtime
}

iheader() {
	printf PGCOPY
	printf '\n'
	printf '\xff'
	printf '\r\n'
	printf '\0'

	printf '\0\0\0\0'
	printf '\0\0\0\0'
}

irow1() {
	printf '\0\x04'

	printf '\0\0\0\x08'
	printf '\0\0\0\0\0\0\0\x01'
	printf '\0\0\0\x08'
	printf '\0\0\0\0\0\0\0\x42'
	printf '\0\0\0\x04'
	printf 'INFO'
	printf '\0\0\0\x04'
	printf 'helo'
}

irow2() {
	printf '\0\x04'

	printf '\0\0\0\x08'
	printf '\0\0\0\0\0\0\0\x02'
	printf '\0\0\0\x08'
	printf '\0\0\0\0\0\0\0\x42'
	printf '\0\0\0\x04'
	printf 'WARN'
	printf '\0\0\0\x04'
	printf 'wrld'
}

idata() {
	irow1
	irow2
	printf '\xff\xff'
}

ipgcopy() {
	iheader
	idata
}

ipgcopy | run_wasi | arrow-cat
#ipgcopy | xxd
