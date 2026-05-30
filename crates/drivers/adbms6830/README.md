# ADBMS6830 Rust Driver


**AI was used to generate the bitfield register maps from the datasheet**

  4. StatusC byte 5 VDE/VDEL. Datasheet Table 91 puts VDEL at bit 7 (MSB) and VDE at bit 6. The C reference parses them swapped. Without hardware to verify, this could be either a datasheet typo or
   a C-ref typo. My impl follows the datasheet — flag this if you see weird VDE/VDEL behavior on the bench.
  6. StatusD byte 4. Datasheet says all-1s filler; C reference parses it as cts:2 | ct:6 (only 8 bits worth). The C driver's std_ struct has ct:6, cts:2 which looks like a copy-paste from an older
  chip — the actual CT[10:0] / CTS[1:0] per Table 91 live in StatusC bytes 2–3 (11+2 = 13 bits). My impl puts them where the datasheet says.
