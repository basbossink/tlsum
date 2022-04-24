<div align="center">
<h1 align="center">tlsum</h1>
<br />
<img alt="License: BSD-2-Clause" src="https://img.shields.io/badge/License-BSD-2-Clause-blue" /><br>
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