![Coverage](https://raw.githubusercontent.com/okx/proof-of-reserves-v2/gh-pages/coverage-badge.svg)

# proof-of-reserves-v2

## Background

OKX launches [Proof of Reserves (PoR)](https://www.okx.com/proof-of-reserves) to improve the security and transparency
of user's assets. These tools will allow you to independently audit OKX's Proof of Reserves and verify OKX's reserves
exceed the exchange's known liabilities to users, in order to confirm the solvency of OKX.


## Liabilities
OKX's PoR uses zk Merkle Sum Tree technology to allow each user to independently review OKX's digital asset reserve on the
basis of protecting user privacy. We used plonky2 to build the proofs of users' assets using merkle sum tree; A detailed documentation of the technical solution is to be given separately.

## run
- gen test data
```
python3 scripts/gen_test_data.py 10 2048
```
- gen mst
```
ENV=local cargo run -p zk-por-core --release --bin gen_mst
```

## code coverage
the code test coverage report is auto generated and hosted at [codecov_report](https://okx.github.io/proof-of-reserves-v2/tarpaulin-report.html) 

## docker
```
docker build -t okx_por_v2 -f docker/Dockerfile .
```

