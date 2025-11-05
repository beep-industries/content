#!/bin/bash

IS_THERE_DOCKER=$(which docker | grep "not found" | xargs)
IS_THERE_PODMAN=$(which podman | grep "not found" | xargs)

if [ "$IS_THERE_DOCKER" != "" ] || [ "$IS_THERE_PODMAN" != "" ]; then
		echo "You need to install docker or podman"
		exit 1
fi
if [ "$IS_THERE_PODMAN" != "" ] && [ "$IS_THERE_DOCKER" == "" ]; then
		echo "Aliasing docker"
		alias podman=docker
fi


CONTAINER_ID=$(podman container list -a --format "{{.ID}} {{.Image}}" | grep "dxflrs/garage" | tr " " "," | sed 's/,.*//g')

NODE_ID=$(podman exec -it $CONTAINER_ID /garage node id)
NODE_ID="${NODE_ID%%@*}"

CLUSTER_LAYOUT=$(podman exec -it $CONTAINER_ID /garage layout show | grep "Current cluster layout version: 0")

if [ "$CLUSTER_LAYOUT" != "" ]; then
		podman exec -it $CONTAINER_ID /garage layout assign -z dc1 -c 1G $NODE_ID > /dev/null
		podman exec -it $CONTAINER_ID /garage layout apply --version 1 > /dev/null
fi

IS_BUCKET_PRESENT=$(podman exec -it $CONTAINER_ID /garage bucket list | grep "beep")
if [ "$IS_BUCKET_PRESENT" == "" ]; then
	podman exec -it $CONTAINER_ID /garage bucket create beep > /dev/null
fi

IS_KEY_PRESENT=$(podman exec -it $CONTAINER_ID /garage key list | grep "beep_admin")
if [ "$IS_KEY_PRESENT" == "" ]; then
		KEY_INFOS=$(podman exec -it $CONTAINER_ID /garage key create beep_admin)
		KEY_ID=$(echo "$KEY_INFOS" | grep "Key ID" | cut -d ":" -f 2 | tr -d " ")
		SECRET_KEY=$(echo "$KEY_INFOS" | grep "Secret key" | cut -d ":" -f 2 | tr -d " ")
		podman exec -it $CONTAINER_ID /garage bucket allow --read --write --owner beep --key beep_admin > /dev/null
		echo "KEY_ID=$KEY_ID"
		echo "SECRET_KEY=$SECRET_KEY"
fi

