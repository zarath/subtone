Subtone
=======

An embedded rust project to generate
[CTCSS](https://en.wikipedia.org/wiki/Continuous_Tone-Coded_Squelch_System)-Tones
via [PDM](https://en.wikipedia.org/wiki/Pulse-density_modulation) on a cheap
[RP2040](https://en.wikipedia.org/wiki/RP2040) board.

![prototype](Prototype.jpg "Prototype attached to 23cm tranciever")

Schematics
----------

Board used is a Seeed Studio XIAO RP2040. When using a Pi-Pico, just use the
corresponding I/O-pins (Px) there.

![schematics](Screenshot.png "XIAO RP2040 Schematics")

Usage
-----

* Use rotary encoder to select wanted frequency.
* Press button short for on/off toggling.
* Press long to store current setting.