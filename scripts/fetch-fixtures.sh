#!/bin/bash

# Update submodules
git submodule update --init --recursive

# cd into protocol-solidity submodules 
cd solidity-fixtures

# run dvc pull
dvc pull -v

# Check if dvc pull succeeds
if [ "$?" -ne 0 ]; then
  echo "dvc pull failed in solidity-fixtures"
  exit 1
fi

# cd out of solidity fixtures 
cd ..

# cd into substrate-fixtures 
cd substrate-fixtures

# run dvc pull
dvc pull -v

# Check if dvc pull succeeds
if [ "$?" -ne 0 ]; then
  echo "dvc pull failed in substrate-fixtures"
  exit 1
fi
