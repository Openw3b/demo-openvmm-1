# OpenOS - OpenVMM
---

OpenVMM can run single apps or it can be configured to run a complete desktop environment in crosvm. The OpenVMM guest agent is a binary which needs to be running inside the guest for mouse, clipboard and window functions.

## Quick start

Please refer to the blog post at https://blog.openw3b.org/crosvm-for-os-and-app-virtualization-on-linux/ for instructions on how to run this demo

## Guest Agent
The guest agent communicates with the host over vsock. When running openvmm the app reuires access to the vhost-vsock device. Give permission to the host user to access the device on the host before running.
```bash
$ sudo chown $USER:$USER /dev/vhost-vsock
```

## Configuration
There are two configurations required by openvmm. The global configuration and the app configuration.
Each app has its yaml file with the configuration of the VM. Here's an example of an app config for running vlc inside the guest.

```yaml
rootfs:
  path: "/path/to/rootfs.qcow2"

display:
  main_window: "vlc"

mounts:
  - /path/to/openvmm/target/debug/openosguest
  - $ARGS[:]

params:
  - "OPENVM_LAUNCH=vlc"
  - $MOUNTS[1:]

networks:
  - kind: "netvm"
    name: "pass"
```

## Display
We tried several approaches to get the display of the app on the host and finally settled on using virtio-gpu for emulating the GPU device inside the guest. This is a feature provided by crosvm when the "gpu" feature is enabled during crosvm compilation. An XWindow is opened on the host which displays the GPU buffer. 
The app in the guest runs inside i3wm. The window manager is configured by default to display borderless windows.

## Sound
Sound is sent from the guest over TCP socket to pulseaudio on the host. The simplest way to do this is to set the `PULSE_SERVER` environment variable for the app to the host IP `10.1.1.1:4713`. Enable the pulseaudio server on the host to accept TCP connections by adding this line in `/etc/pulse/default.pa`.
```
load-module module-native-protocol-tcp auth-ip-acl=10.1.1.0/24
```


## Mouse
The mouse movements are emulated inside the VM by the guest agent. The mouse X events are sent to the guest agent over vsock and the agent moves the mouse inside the guest using xdotool.

## Clipboard
Clipboard is sent to the guest only when needed. When the user presses ctrl+v and the app window is focused. the host sends the clipboard content from the host to the guest agent and the agent simulates the same key combination on the guest. Similarly for ctrl+c the host requests the clipboard from the guest and the content is copied to the hosts clipboard.

## File Input 
Files are sent to the guest using virtio-fs shared directory. By default openvmm creates a directory in `/tmp/\<random_dir\>` for each running instance. The `mount` directory inside this temporary directory is mounted inside the guest at /mnt. The mounts parameter in the app configuration is a list of files which are hard linked inside this directory for the guest to access. This way the guest cannot see any othe files on the host except what is specified. For apps which require access to the files passed as argument while running the app the `$ARGS[:]` placeholder can be used. This would expect all arguments to openvmm when running the app to be file paths and openvmm would mount them. For specific argument or a range of arguments provide the start (inclusive) and end (exclusive) of range `$ARGS[<start>:<end>]`. The vlc.yaml file configuration demonstrates this feature.