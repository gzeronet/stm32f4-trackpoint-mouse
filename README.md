# stm32f4-trackpoint-mouse

## Let trackpoint module works on stm32f4 with rust embedded

![stm32_trackpoint](https://github.com/gzeronet/stm32f4-trackpoint-mouse/assets/4970608/368f5f5d-2004-41ab-b564-7431b229c68e)

https://github.com/gzeronet/stm32f4-trackpoint-mouse/assets/4970608/e9068c04-86c7-41cc-81bf-1a115b81c6c1

---

Several years ago, I built a trackpoint keyboard (use tmk repo and convert [arduino-trackpoint-extended](https://github.com/rampadc/arduino-trackpoint-extended) to c).

Recently, I lost my python job, but I feel so happy to start my own project: Rebuild/Rewrite my trackpoint keyboard in rust :D.

[Legacy c version](https://github.com/gzeronet/teensy-trackpoint-tmk-keyboard)

---

## Why Rebuild?

* Replaced developing board, teensy 2.0 board is too expensive, stm32f401 is cheaper enough.
* Want to learn rust with its embedded developing to improve my code language kills, seems no such sample/crate for teensy 2.
* Feels rust embedded is easier than c/c++ to learn. Just follow stmcubeide to build c projects, it drives me crazy.
* I like trackpoint than any other mouse hid, but not found firmware in rust version.

So, I tried rewriting the module in rust, and it works perfect as I wanted now.

---

## Feature

* Use stm32f4xx_hal, rtic, ps/2 to usb.
* TIM EXTI, GPIO EXTI.
* Trackpoint stream mode, (remote mode works, but I don't need).

---

## Build

> cargo objcopy --bin trackpoint_mouse -- -O binary trackpoint_mouse.bin

> dfu-util -d 0483:df11 -a 0 --dfuse-address 0x08000000 -D trackpoint_mouse.bin

---

## Updated

Use open drain mode now, need pull up SDA & SCL with 4.7k resister for each other.

---

## TODO

Continue working with trackpoint keyboard in rust & stm32f4...

Include keyoard code part like [keyberon](https://github.com/TeXitoi/keyberon), maybe will adjust the repo name.
