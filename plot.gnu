#!/usr/bin/gnuplot
set datafile separator ","
set timefmt "%Y-%m-%dT%H:%M:%S"

set title 'Particulates'
set grid

set xdata time
set xtics time
set format x '%H:%M:%S'
set xlabel 'Time'

set ylabel 'ug/m3'
set yrange [0:*]
set ytics nomirror
set y2tics
set y2range [0:*]

plot 'dump-sps30.csv' using 1:3 with lines title 'PM2.5', 'dump-sps30.csv' using 1:5 with lines title 'PM10', 'dump-8020a.csv' using 1:2 with lines title 'P8020A' axis x1y2

# Live reload
pause 5
reread
