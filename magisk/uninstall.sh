#!/system/bin/sh
killall niupanel 2>/dev/null

CHROOT_DIR="/data/adb/niupanel/debian"
umount -l $CHROOT_DIR/app 2>/dev/null
umount -l $CHROOT_DIR/etc/hosts 2>/dev/null
umount -l $CHROOT_DIR/etc/resolv.conf 2>/dev/null
umount -l $CHROOT_DIR/proc 2>/dev/null
umount -l $CHROOT_DIR/sys 2>/dev/null
umount -l $CHROOT_DIR/dev/pts 2>/dev/null
umount -l $CHROOT_DIR/dev 2>/dev/null

rm -rf "/data/adb/niupanel"
