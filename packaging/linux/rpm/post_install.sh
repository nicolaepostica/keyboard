# Applies the uinput udev rule now, so /dev/uinput belongs to the `input` group without a reboot.
if command -v udevadm >/dev/null 2>&1; then
    udevadm control --reload-rules || :
    udevadm trigger --subsystem-match=misc --sysname-match=uinput || :
fi
