# atari2600

An Atari 2600 emulator, because I like stepping back in time to the console wars of the 80s.

The goal of this is to be able to play a couple of key games that have sentimental value to me, rather than attempting to be a highly accurate emulator that can play the vast library of games available. For that, there's [Stella](https://github.com/stella-emu/stella).

# TODO

This is still in progress. It can play games, but it's still got problems.

1. ~~Docs~~ ([this](https://problemkaputt.de/2k6specs.htm) is great)
2. ~~6507 CPU~~ (rip out the interrupts from my 6502)
3. ~~Basic memory map~~ (13-bit address bus, RAM, and cartridge ROM)
4. ~~SDL integration~~
5. ~~Basic TIA frame timing~~
6. ~~Playfield rendering~~
7. ~~Missle graphics~~
8. ~~Ball graphics~~
9. ~~Player graphics~~
10. ~~Horizontal positioning and HMOVE~~
11. ~~Graphics delay on LRHB~~
12. ~~Vertical delay~~
13. ~~Collision flags~~
14. Fix frame timing
15. ~~RIOT chip (MOS 6532) for peripherals~~
16. Fix small horizontal position bugs
17. Audio (see [this](https://www.biglist.com/lists/stella/archives/200311/msg00156.html))
18. ..
19. Adventure time!

# Building and Runnning

```
$ brew install sdl2
$ cargo build --release
$ target/release/atari2600 roms/Pitfall.a26
```

# Console Buttons

| Console Switch | Keyboard Button |
| -------------- | --------------- |
| Select | F1 |
| Reset | F2 |
| Color Toggle | F3 |

# Joystick 1 Keys

| Joystick Button | Keyboard Button |
| --------------- | --------------- |
| Up | W |
| Left | A |
| Down | S |
| Right | D |
| Button | N |
