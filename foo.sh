#!/usr/bin/env bash

set -eu
set -o pipefail

if (cargo test --color always -- -q --test-threads=1 |& tee gean) ; then
  echo ==========
  echo STAGING!!!
  echo ==========
  git add src
else
  git checkout .
fi

git status
