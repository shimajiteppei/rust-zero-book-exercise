# rust-zero-book-exercise

[ゼロから学ぶ Rust システムプログラミングの基礎から線形型システムまで](https://www.kspub.co.jp/book/detail/5301951.html)の勉強用リポジトリ

## メモ

[ytakano/rust_zero](https://github.com/ytakano/rust_zero)

### 6 章

`cargo install cargo-criterion`

- [Regular Expression Matching Can Be Simple And Fast](https://swtch.com/~rsc/regexp/regexp1.html)
- [Regular Expression Matching: the Virtual Machine Approach](https://swtch.com/~rsc/regexp/regexp2.html)

Cox の正規表現`(a?){n}a{n}`は`?`によって ε 遷移のループが指数関数的に生じる。

https://swtch.com/~rsc/regexp/regexp1.html の Performance に書かれているが、n が増加するごとに深さ優先実装では実行時間が指数関数的に増えるのに対して、幅優先実装は対数関数的に増える。

[OWASP の ReDoS 紹介ページ](https://owasp.org/www-community/attacks/Regular_expression_Denial_of_Service_-_ReDoS)に掲載されていた正規表現`(a|a?)+`を試すと、本の実装では深さ優先探索の再帰処理がスタックオーバーフローした。

スタックオーバーフロー回避ため深さ優先、幅優先ともに、再帰処理での実装はしない。また、レジスタマシンの状態をキャッシュして探索済みの状態の評価をカットする。

ベンチマークの結果では、Cox の正規表現`(a?){n}a{n}`では大きな違いが見られなかったが、`(a+)+`や`(a|a?)+`のネストした無限ループを持つ正規表現では、対数関数と指数関数の違いが明確になっている。

```
Width First/Cox:n=02            time:   [523.57 ns 533.21 ns 545.32 ns]
Width First/Cox:n=04            time:   [1.4912 µs 1.4954 µs 1.4999 µs]
Width First/Cox:n=08            time:   [4.9021 µs 4.9157 µs 4.9328 µs]
Width First/Cox:n=16            time:   [12.894 µs 12.944 µs 12.995 µs]
Width First/Cox:n=32            time:   [45.632 µs 45.790 µs 45.979 µs]
Width First/Cox:n=64            time:   [171.79 µs 171.86 µs 171.94 µs]
Width First/Cox:n=128           time:   [672.72 µs 673.29 µs 674.00 µs]

Width First/nested plus:n=02    time:   [496.60 ns 498.13 ns 499.71 ns]
Width First/nested plus:n=04    time:   [499.69 ns 500.11 ns 500.56 ns]
Width First/nested plus:n=08    time:   [534.01 ns 534.45 ns 534.92 ns]
Width First/nested plus:n=16    time:   [552.69 ns 553.10 ns 553.53 ns]
Width First/nested plus:n=32    time:   [582.28 ns 582.90 ns 583.59 ns]
Width First/nested plus:n=64    time:   [581.33 ns 581.83 ns 582.33 ns]
Width First/nested plus:n=128   time:   [625.51 ns 629.32 ns 633.24 ns]

Width First/Cox like:n=02       time:   [631.69 ns 632.36 ns 633.03 ns]
Width First/Cox like:n=04       time:   [644.86 ns 647.10 ns 650.58 ns]
Width First/Cox like:n=08       time:   [676.06 ns 676.73 ns 677.41 ns]
Width First/Cox like:n=16       time:   [735.56 ns 736.20 ns 736.86 ns]
Width First/Cox like:n=32       time:   [748.29 ns 749.01 ns 749.71 ns]
Width First/Cox like:n=64       time:   [820.94 ns 822.19 ns 823.55 ns]
Width First/Cox like:n=128      time:   [832.49 ns 833.61 ns 834.74 ns]

Depth First/Cox:n=02            time:   [548.82 ns 550.33 ns 552.16 ns]
Depth First/Cox:n=04            time:   [1.5489 µs 1.5503 µs 1.5516 µs]
Depth First/Cox:n=08            time:   [4.9068 µs 4.9102 µs 4.9137 µs]
Depth First/Cox:n=16            time:   [12.916 µs 12.926 µs 12.938 µs]
Depth First/Cox:n=32            time:   [45.130 µs 45.163 µs 45.196 µs]
Depth First/Cox:n=64            time:   [176.34 µs 176.45 µs 176.58 µs]
Depth First/Cox:n=128           time:   [712.37 µs 712.91 µs 713.52 µs]

Depth First/nested plus:n=02    time:   [490.94 ns 492.72 ns 494.71 ns]
Depth First/nested plus:n=04    time:   [667.50 ns 668.59 ns 669.71 ns]
Depth First/nested plus:n=08    time:   [1.0489 µs 1.0507 µs 1.0524 µs]
Depth First/nested plus:n=16    time:   [1.7348 µs 1.7406 µs 1.7488 µs]
Depth First/nested plus:n=32    time:   [3.2136 µs 3.2186 µs 3.2250 µs]
Depth First/nested plus:n=64    time:   [5.5670 µs 5.5777 µs 5.5895 µs]
Depth First/nested plus:n=128   time:   [10.106 µs 10.116 µs 10.128 µs]

Depth First/Cox like:n=02       time:   [968.74 ns 970.15 ns 971.55 ns]
Depth First/Cox like:n=04       time:   [1.5069 µs 1.5110 µs 1.5144 µs]
Depth First/Cox like:n=08       time:   [2.5547 µs 2.5572 µs 2.5597 µs]
Depth First/Cox like:n=16       time:   [4.5035 µs 4.5085 µs 4.5150 µs]
Depth First/Cox like:n=32       time:   [8.1064 µs 8.1120 µs 8.1174 µs]
Depth First/Cox like:n=64       time:   [15.883 µs 15.897 µs 15.912 µs]
Depth First/Cox like:n=128      time:   [30.281 µs 30.303 µs 30.326 µs]
```

### 7 章

- https://qiita.com/ko1nksm/items/5018649160820006bdf6

POSIX シェルで定義されているビルトインコマンド https://pubs.opengroup.org/onlinepubs/9699919799/utilities/V3_chap02.html#tag_18_09_01

- special built-in utility
  - break
  - :
  - continue
  - .
  - eval
  - exec
  - exit
  - export
  - readonly
  - return
  - set
  - shift
  - times
  - trap
  - unset
- built-in utility
  - alias
  - bg
  - cd
  - command
  - false
  - fc
  - fg
  - getopts
  - hash
  - jobs
  - kill
  - newgrp
  - pwd
  - read
  - true
  - umask
  - unalias
  - wait
