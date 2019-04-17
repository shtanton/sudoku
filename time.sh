head -n 10 alot.txt | xargs -I % -n 1 sh -c "echo % | ./target/release/sudoku nine.fmt"
