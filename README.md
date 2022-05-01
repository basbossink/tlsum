<div align="center">
<h1 align="center">tlsum</h1>
<br />
<img alt="License: BSD-2-Clause" src="https://img.shields.io/badge/License-BSD-blue" /><br>
<br>
An emacs timelog summarizer.
</div>

***
This small application takes a timelog file as created by the
[Emacs](https://www.gnu.org/software/emacs/) `M-x timeclock-in` and
`M-x timeclock-out` commands, and provides the following summary information:
- The number of days worked; the number of unique dates that have a clock in (`i`) event.
- The total number of hours and minutes clocked.
- The average number of hours and minutes clocked per day.
- The cummulative overtime up to but not including the last date there was a clock in, typically yesterday.
- The first clock in of today.
- The number of hours worked today.
- The number of hours and minutes still to work today, taking overtime into account.
- The number of hours and minutes still to work today, based on an 8 hour workday today.
- The time to leave, taking overtime into account.
- The time to leave, based on an 8 hour workday today.
  
`tlsum` assumes an 8 hour workday, any time alotted for lunch breaks is not taken into account for now.

The excellent [ledger-cli](https://www.ledger-cli.org/), can create some nice 
reports for the timelog as well I strongly recommend using it, refer to the 
[documentation here](https://www.ledger-cli.org/3.0/doc/ledger3.html#Time-Keeping).
This tool is merely a [Rust](https://www.rust-lang.org/) learning project,
and a reimplementation of the [fish](https://fishshell.com/)
and [awk](https://en.wikipedia.org/wiki/AWK)
scripts in my [dot-files](https://github.com/basbossink/dot-files-via-chezmoi).

### Installation
```
cargo install tlsum
```

### Usage
```
TIMELOG="$HOME/.emacs.d/timelog" tlsum
```

### License
This project is licensed under the BSD-2-Clause license. See the [LICENSE](LICENSE) for details.

***
Readme made with 💖 using [README Generator by Dhravya Shah](https://github.com/Dhravya/readme-generator)