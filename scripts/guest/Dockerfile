FROM ubuntu

RUN apt-get update && \
    apt-get install -y dhcpcd5 util-linux systemd systemd-sysv
RUN DEBIAN_FRONTEND=noninteractive apt-get install -y xorg
RUN apt-get install -y i3-wm xterm sudo xss-lock netplan.io nano net-tools inetutils-ping iproute2 xdotool

RUN systemctl disable gdm

ARG PACKAGES='firefox vlc'
# ARG COMMAND=firefox

RUN DEBIAN_FRONTEND=noninteractive apt-get install -y $PACKAGES

COPY files/init /init
RUN chmod +x /init

COPY files/netplan.yaml /etc/netplan/00-default.yaml
RUN systemctl disable dhcpcd

RUN useradd -s /bin/bash -d /home/user/ -m -G sudo user
RUN chown -R user:user /home/user

# Auto resize
#COPY files/xrandrloop.sh /opt/xrandrloop.sh
#RUN chmod +x /opt/xrandrloop.sh

# Autologin tty0 with user 
RUN mkdir -p /etc/systemd/system/getty@tty1.service.d
COPY files/override.conf /etc/systemd/system/getty@tty1.service.d/override.conf

COPY files/.xinitrc /home/user/.xinitrc
COPY files/config /etc/i3/config
COPY files/.bash_profile /home/user/.bash_profile

#RUN echo $COMMAND /mnt/example.mp4 > /opt/app.sh
# RUN echo $COMMAND > /opt/app.sh
COPY files/app.sh /opt/app.sh
RUN chmod +x /opt/app.sh
RUN echo 'exec --no-startup-id /opt/app.sh' >> /etc/i3/config

# Start i3 on boot
#COPY files/app.service /etc/systemd/system/app.service
#COPY files/app.service /lib/systemd/system/app.service
#RUN chmod 644 /etc/systemd/system/app.service
#RUN systemctl enable app


# Install ROC Toolkit
