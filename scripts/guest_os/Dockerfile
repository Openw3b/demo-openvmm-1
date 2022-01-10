FROM ubuntu

RUN apt-get update && \
    apt-get install -y dhcpcd5 util-linux systemd systemd-sysv
RUN DEBIAN_FRONTEND=noninteractive apt-get install -y xorg
RUN apt-get install -y sudo netplan.io nano net-tools inetutils-ping iproute2 xdotool

COPY files/init /init
RUN chmod +x /init

COPY files/netplan.yaml /etc/netplan/00-default.yaml
RUN systemctl disable dhcpcd

RUN useradd -s /bin/bash -d /home/user/ -m -G sudo user
RUN chown -R user:user /home/user

# Start openosguest on boot
COPY files/openos.guest.desktop /etc/xdg/autostart/openos.guest.desktop
COPY files/gdm.custom.conf /etc/gdm3/custom.conf
# COPY files/app.service /lib/systemd/system/app.service
# RUN chmod 644 /etc/systemd/system/app.service
# RUN systemctl enable app