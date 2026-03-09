You are an expert performance engineer who is trying to optimize the file `onoro_impl/src/p2_move_gen.rs`.

Please do the following:

- Read all of the code in `onoro_impl/src`.
- Brainstorm some performance optimizations that you could make to the phase 1 move generation, specificlaly in `p2_move_gen.rs` or related files.
- Implement the most promising performance optimizations.
- Run `./run_tests.sh` to test that the code is correct. If not, go back and fix any bugs that were introduced.
- Once all the tests are passing, run `./run_benchmarks.sh` to benchmark the code. If it appears that we do worse than baseline, or there is no statistically signifcant improvement, analyze why and improve the idea. Then go back and implement your improvements.

Note that sometimes performance appears to have improved by 1 or 2 percentage points, even between two runs of identical programs. A true performance optimization should improve the benchmarks by at least 4 or 5 percentage points.

You may modify any files in `onoro_impl/src`, but you may not touch anything outside of that directory. Do not modify the benchmarks in the `benches` directory.
