# rust-zero-book-exercise

[ゼロから学ぶ Rust システムプログラミングの基礎から線形型システムまで](https://www.kspub.co.jp/book/detail/5301951.html)の勉強用リポジトリ

## メモ

[ytakano/rust_zero](https://github.com/ytakano/rust_zero)

### 6 章

`cargo install cargo-criterion`

- [Regular Expression Matching Can Be Simple And Fast](https://swtch.com/~rsc/regexp/regexp1.html)
- [Regular Expression Matching: the Virtual Machine Approach](https://swtch.com/~rsc/regexp/regexp2.html)

Cox の正規表現`(a?){n}a{n}`は`?`によって ε 遷移のループが指数関数的に生じる。

幅優先探索は状態をキャッシュすることで探索済みの状態の評価をカットできるので実行時間がかなり削減される。いろいろな既存の実装との比較は https://swtch.com/~rsc/regexp/regexp1.html の Performance に書かれている。n が増加するごとに、深さ優先実装では実行時間が指数関数的に増えるのに対して、幅優先実装は対数関数的に増える。

```
Width First/n=02        time:   [680.88 ns 684.10 ns 687.54 ns]
Width First/n=04        time:   [2.0093 µs 2.0113 µs 2.0132 µs]
Width First/n=08        time:   [6.6163 µs 6.6205 µs 6.6245 µs]
Width First/n=16        time:   [19.281 µs 19.289 µs 19.296 µs]
Width First/n=32        time:   [68.139 µs 68.499 µs 68.948 µs]
Width First/n=64        time:   [270.72 µs 272.00 µs 273.05 µs]
Width First/n=128       time:   [1.0453 ms 1.0536 ms 1.0614 ms]

Depth First/n=02 #2     time:   [215.82 ns 216.15 ns 216.55 ns]
Depth First/n=04 #2     time:   [506.61 ns 507.07 ns 507.64 ns]
Depth First/n=08 #2     time:   [4.0479 µs 4.0490 µs 4.0502 µs]
Depth First/n=16 #2     time:   [1.2058 ms 1.2076 ms 1.2097 ms]
Benchmarking Depth First/n=32 #2: Warming up for 3.0000 s
Warning: Unable to complete 100 samples in 12.0s. You may wish to increase target time to 14984.8s, or reduce sample count to 10.
Benchmarking Depth First/n=32 #2: Collecting 100 samples in estimated  14985 s (100 iterations)^C
```
