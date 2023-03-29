# LargeDataSorter

Generating a file with random numbers:
`cargo run --release -- -g <number of 32-bit unsigned integers> <output filepath>`

Sorting a file:
`cargo run --release -- -s <input filepath> <output filepath> [number of threads]`
where numbers of threads should (but not have to) be the power of 2 