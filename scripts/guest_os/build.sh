#!/bin/bash
# sudo rm fs/fs.tar fs/${APP_NAME}.qcow2

set -e 
pushd "$(dirname "${BASH_SOURCE[0]}")"

# For virt-make-fs
# apt install --yes --no-install-recommends libguestfs-tools 

GUESTFS="../../config/guestfs"
mkdir -p ${GUESTFS}
if [ ! -f "../../config/guestfs/rootfs_os.tar" ]; then
    DOCKER_BUILDKIT=1 docker build --output "type=tar,dest=${GUESTFS}/rootfs_os.tar" . 
    # virt-make-fs --format=qcow2 --size=+1G ${GUESTFS}/rootfs.tar ${GUESTFS}/rootfs.qcow2
fi

popd