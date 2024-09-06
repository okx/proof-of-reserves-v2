# Config
edit `config/default.toml` such that `user_data_path` equal to the directory which contains only the user files. Currently, it is set to `./sample_data` for demo. 

# Prove
```
cfg_dir_path="config"

output_proof_dir_path="proof"
./zk-por-prover prove --cfg-path ${cfg_dir_path} --output-path ${output_proof_dir_path}
```

# Verify
## Verify only global proof
For internal use. 

## Batch-verify user proofs
For internal use. 

## Global proof and user proofs
For external use
```

```