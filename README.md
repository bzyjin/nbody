# nbody

This is a sketch that simulates `n` "particles" in 2 dimensions.

## Details

The sketch (by default) computes a gravitational field approximation.

It is expected to run in roughly linear time (`n log log n` for tree construction and `n` for attraction) for uniform point distributions, but the default simulation spawns tightly clustered particles which can degrade performance (I might fix this in the future).

The field approximation algorithm is an oversimplified [FMM](https://en.wikipedia.org/wiki/Fast_multipole_method). Collision is not implemented.

[Video](https://github.com/user-attachments/assets/28dc8e14-b4eb-4737-8f75-e795a476a245)
