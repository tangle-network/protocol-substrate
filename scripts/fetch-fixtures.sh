#!/bin/bash

# run dvc pull
dvc pull -v

# Check if dvc pull succeeds
if [ "$?" -ne 0 ]; then
  echo "dvc pull failed"
  exit 1
fi
