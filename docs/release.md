# Config
Edit `config/default.toml` such that `user_data_path` is the directory containing the user files only. Currently, it is set to `./sample_data` for demo. 

# Prove
```
cfg_dir_path="config"
output_proof_dir_path="proof"

./zk-por-prover prove --cfg-path "${cfg_dir_path}" --output-path "${output_proof_dir_path}"
```
In the current directory, a directory `proof` is generated with the below files:
```
- global_proof.json # the sum and non-negative proof
- global_info.json # contains the root hash, the sum of equity, debt and balance (equity - debt)
- user-proofs/ # directory containing user inclusion proofs, one user per file named with accountID
  - xxxxxxx.json
```

# Verify
## Verify only global proof
For internal use. 
```
global_proof_path="./proof/global_proof.json"

./zk-por-verifier verify-global --proof-path "${global_proof_path}"
```
If successful, the console shows
```
start to reconstruct the circuit with 1 recursive levels for round 0
successfully reconstruct the circuit for round 0 in 19.930038084s
successfully verify the global proof for round 0, total exchange users' equity is 8801210029, debt is 875, exchange liability is 8801209154
```

## Batch-verify user proofs
For internal use. 
```
user_proof_path_pattern="./proof/user_proofs/*.json" # use wildcard to verify multiple files

./zk-por-verifier verify-user --global-proof-path "${global_proof_path}" --user-proof-path-pattern "${user_proof_path_pattern}"
```
If successful, the console shows
```
successfully identify 8 user proof files
█████████████████████████████████████████████████████████████████████████████████████████████ 8/8
8/8 user proofs pass the verification. 0 fail, the first 0 failed proof files: []
Execution result: Ok(())
```

## Global proof and user proofs
For external users to verify, 
```
# the binary will look for global_proof.json and any files with *_inclusion_proof.json in the same directory for verification. So we first copy them to the current directory. 

cp proof/global_proof.json ./global_proof.json
# copy any one of user proofs. 
cp proof/user_proofs/$(ls proof/user_proofs/ | head -n 1) ./user_inclusion_proof.json

./zk-por-verifier
```
If successful, the console shows:
```
============Validation started============
Total sum and non-negative constraint validation passed
Inclusion constraint validation passed
============Validation finished============
```
If any proof fails to be verified, the console shows:
```
============Validation started============
Total sum and non-negative constraint validation passed
Inclusion constraint validation failed
============Validation finished============
```