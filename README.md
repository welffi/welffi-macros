# welffi macros

simple prebuilt macros and a recorder. part of [welffi hub](https://github.com/welffi/welffi-hub)

## what it does

- ctd easy: runs the easy loop by itself, and works across more than one roblox window
- ctd easy clear: the same loop on one window. Setup stops early as soon as a results screen shows, won or lost, so a long setup recording is fine
- recorder: record your mouse and keys, play them on loop, and save or load them
- sizes the roblox window for you and leaves fullscreen if needed
- can shut down your pc if roblox shows the disconnected screen while ctd runs. it waits 60 seconds first, and `shutdown /a` cancels it
- can shut down your pc after a set number of runs. it waits 60 seconds first, and `shutdown /a` cancels it
- counts runs, wins and losses for each start
- rejoin: pick an account and a private server from the ctd page. the accounts and servers come from welffi accounts, and you can add a server link here too. it works on one roblox window. if roblox shows the disconnected screen, it closes the game and joins that private server again the same way welffi accounts does
- restart retry: 10 seconds after the restart, it looks for the cancel button until start_game shows. if cancel is there, it plays restart_retry and checks again 10 seconds later
- lobby recovery: if neither start_game nor results shows for 30 seconds, it presses esc, r and enter again and again until the toilet city image shows, then plays enter_toilet_city. if toilet city doesn't show for 2 minutes, it rejoins and keeps trying. with no server picked it only keeps resetting. a picked server replaces shut down if disconnected

## keys

- f2: start or stop
- f3: stop
- ctrl+r and ctrl+p: record and play, in the recorder
- ctrl+f1: show or hide the window
- ctrl+esc: exit

## how to download

get it from welffi hub, or get `welffi-macros.exe` from the [latest release](https://github.com/welffi/welffi-macros/releases/latest) and run it. nothing else needs installing
