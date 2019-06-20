start=$(date)
head -n 5 alot.txt | ./target/release/sudoku nine.fmt
echo $start
date
