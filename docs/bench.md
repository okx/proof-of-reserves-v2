# circuit benchmarking
## range checking
checked nums size: 1024*220, which is `batch_size` * `num_of_tokens`


to reproduce results

on local mac: Apple M2 Max, with 32 GB RAM
```
cargo bench -p zk-por-core --bench rangecheck_lut
```

### do range checking by split into bits

225280 `BaseSumGate` is used to check the splited bits can sum to the original value

Takes **23.091** seconds to prove

### do range checking by split into 3 16bits limb; and use lookup table to check each limb

22528 `ArithmeticGate` is used to check the 3 limbs can sum to the original value
one look up table with `1<<16` elements.

Takes **1.2558** seconds to prove