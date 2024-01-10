#!/bin/bash -eu

# NOUSE: see tag.py

die () {
	msg="${2:-}"
	[[ -n "$msg" ]] && echo "$msg"
	exit "$1"
}

create () {
	local file="$1"
	shift
	local tags=($@)
	hash="$(git hash-object "$file")"
	out="$(echo "insert into object values ('${hash}');"|sqlite3 tag.db 2>&1)" || {
		echo "$out"|grep -q 'UNIQUE constraint failed' || die 1 "$out"
	}
	object_oid="$(echo "select oid from object where hash='${hash}';"|sqlite3 tag.db 2>&1)"
	for tag in "${tags[@]}"; do
		out="$(echo "insert into tag values ('${tag}');"|sqlite3 tag.db 2>&1)" || {
			echo "$out"|grep -q 'UNIQUE constraint failed' || die 1 "$out"
		}
		tag_oid="$(echo "select oid from tag where name='${tag}';"|sqlite3 tag.db 2>&1)"
		out="$(echo "insert into object_tag values ('${object_oid}', '${tag_oid}');"|sqlite3 tag.db 2>&1)" || {
			echo "$out"|grep -q 'UNIQUE constraint failed' || die 1 "$out"
		}
	done
}

_join () {
	local IFS="$1"
	shift
	a=()
	for v in "$@";do
		a+=("'${v}'")
	done	
	echo "${a[*]}"
}

_find () {
	tag_id="$(echo "select oid from tag where name in (${1});"|sqlite3 tag.db)"
	[[ -z "$tag_id" ]] && die 2 "No tags found for ${1}"
	object_id="$(echo "select object_id from object_tag where tag_id='${tag_id}';"|sqlite3 tag.db)"
	hash="$(echo "select hash from object where oid='${object_id}';"|sqlite3 tag.db)"
	git ls-tree -r @ |grep "$hash"|awk '{print $NF}'
}

_post_commit() {
	git diff --name-status @^ \

	#| awk '/^A|D|
}

cmd="$1"

case "$cmd" in
	create)
		file="$2"
		shift
		create "$file" $@
		;;
	find)
		shift
		tags="$(_join , $@)"
		_find "$tags"
		;;
	post-commit)
		_post_commit
		;;
	*)
		die 1 "$1"
		;;
esac
