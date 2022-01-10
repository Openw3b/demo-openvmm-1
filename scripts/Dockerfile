ARG RUST_VERSION=1.56.1
ARG HOME
FROM docker.io/rust:${RUST_VERSION}-slim-bullseye

RUN useradd -ms /bin/bash -G sudo user
# Use a dedicated target directory so we do not write into the source directory.
# RUN mkdir -p /scratch/cargo_target
# ENV CARGO_TARGET_DIR=/scratch/cargo_target

# Prevent the container from writing root-owned __pycache__ files into the src.
ENV PYTHONDONTWRITEBYTECODE=1

# Install dependencies for native builds.
COPY scripts/install-deps /scripts/
RUN apt-get update \
    && apt-get install --yes sudo \
    && /scripts/install-deps
    # Clear apt cache to save space in layer.
    # && rm -rf /var/lib/apt/lists/* \
    # # Delete build artifacts from 'cargo install' to save space in layer.
    # && rm -rf /scratch/cargo_target/*

# RUN virt-make-fs --format=qcow2 --size=+1G /workspace/config/guestfs/rootfs.tar /workspace/config/guestfs/rootfs.qcow2

# Build OpenVMM
ADD . /workspace/
WORKDIR /workspace
RUN cargo build

# RUN chown -R user:user /workspace
# env USER=user
# RUN chown root:root /workspace/target/debug/opennetutil && \
#     chmod u+s /workspace/target/debug/opennetutil && \
#     chmod g+s /workspace/target/debug/opennetutil

# RUN echo 'user:password' | sudo chpasswd
# RUN groupadd -g 108 kvm
# RUN usermod -a -G kvm user
# RUN chown user:user /dev/vhost-vsock

# USER user