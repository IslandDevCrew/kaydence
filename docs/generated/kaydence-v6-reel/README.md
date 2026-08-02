# Kaydence v6 executive reel source

Five six-second SVG acts form the 30-second motion argument embedded in the
enhanced State of the Union + Plan of Attack v6 report. Each frame is sourced
from `ops/mission/state.json`, the August 2 GitHub Actions run, and the P1 gate
evidence. Rendered output is deterministic and contains no external assets.

Render command:

```bash
for n in 01 02 03 04 05; do
  sips -s format png "act-$n.svg" --out "act-$n.png"
done

ffmpeg -y -framerate 30 -loop 1 -t 6 -i act-01.png \
  -framerate 30 -loop 1 -t 6 -i act-02.png \
  -framerate 30 -loop 1 -t 6 -i act-03.png \
  -framerate 30 -loop 1 -t 6 -i act-04.png \
  -framerate 30 -loop 1 -t 6 -i act-05.png \
  -filter_complex '[0:v][1:v][2:v][3:v][4:v]concat=n=5:v=1:a=0,format=yuv420p[v]' \
  -map '[v]' -r 30 -movflags +faststart Kaydence-Executive-Reel-v6-2026-08-02.mp4
```
