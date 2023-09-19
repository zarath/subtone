Subtone
=======

An embedded rust project to generate
[CTCSS](https://en.wikipedia.org/wiki/Continuous_Tone-Coded_Squelch_System)-Tones
via [PDM](https://en.wikipedia.org/wiki/Pulse-density_modulation) on a cheap
[RP2040](https://en.wikipedia.org/wiki/RP2040) board.
Due to PDM (aka Delta-Modulation) a simple rc-filter is sufficient for
DA-conversation.
(eg. 100Ω/3.3µF if not using 1750Hz option or 100Ω/470nF otherwise)
Prefer tantalum caps for better switching supression.

![prototype](Prototype.jpg "Prototype attached to 23cm tranciever")

Waveform Example
----------------

![waveform](Oscilloscope.png "Waveform example on oscilloscope")

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
* With 1750Hz call-tone option on this frequency short button push just emmits
  a one second sine pulse and is quiet otherwise.