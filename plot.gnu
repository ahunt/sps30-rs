#!/usr/bin/gnuplot
set datafile separator ","
set title 'Particulates'
set ylabel 'ug/m3'
set xlabel 'Time'
set timefmt "%Y-%m-%dT%H:%M:%SZ"
set xdata time
set grid

plot 'out.csv' using 0:2 with lines title 'PM2.5', 'out.csv' using 0:4 with lines title 'PM10'

# Live reload
pause 5
reread
