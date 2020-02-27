# GDB file for Black Magic Probe
# You'll need to change the device below to the USB path of your probe
target extended-remote /dev/cu.usbmodemD6D286E11

monitor tpwr enable
monitor swdp_scan
attach 1

# print demangled symbols
set print asm-demangle on
set print pretty

# set backtrace limit to not have infinite backtrace loops
set backtrace limit 32

# detect unhandled exceptions, hard faults and panics
break DefaultHandler
break HardFault
break rust_begin_unwind
# # run the next few lines so the panic message is printed immediately
# # the number needs to be adjusted for your panic handler
# commands $bpnum
# next 4
# end

# *try* to stop at the user entry point (it might be gone due to inlining)
#break main

load

# start the process but immediately halt the processor
stepi
