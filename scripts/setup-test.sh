#!/bin/bash

file_num=10
per_file_account_num=8192 # multiple of 1024, the batch size

# test data will be generated to ./test-data/user-data
rm -rf ./test-data/user-data
mkdir -p ./test-data/user-data
python3 scripts/gen_test_data.py ${file_num} ${per_file_account_num}
