#!/bin/bash

CONTAINER_ID=$(docker container list -a --format "{{.ID}} {{.Image}}" | grep "dxflrs/garage" | tr " " "," | sed 's/,.*//g')

NODE_ID=$(docker exec -it $CONTAINER_ID /garage node id)
NODE_ID="${NODE_ID%%@*}"

CLUSTER_LAYOUT=$(docker exec -it $CONTAINER_ID /garage layout show | grep "Current cluster layout version: 0")

if [ "$CLUSTER_LAYOUT" != "" ]; then
		docker exec -it $CONTAINER_ID /garage layout assign -z dc1 -c 1G $NODE_ID > /dev/null
		docker exec -it $CONTAINER_ID /garage layout apply --version 1 > /dev/null
fi

IS_BUCKET_PRESENT=$(docker exec -it $CONTAINER_ID /garage bucket list | grep "beep")
if [ "$IS_BUCKET_PRESENT" == "" ]; then
	docker exec -it $CONTAINER_ID /garage bucket create beep > /dev/null
fi

IS_KEY_PRESENT=$(docker exec -it $CONTAINER_ID /garage key list | grep "beep_admin")
if [ "$IS_KEY_PRESENT" == "" ]; then
		KEY_INFOS=$(docker exec -it $CONTAINER_ID /garage key create beep_admin)
		KEY_ID=$(echo "$KEY_INFOS" | grep "Key ID" | cut -d ":" -f 2 | tr -d " ")
		SECRET_KEY=$(echo "$KEY_INFOS" | grep "Secret key" | cut -d ":" -f 2 | tr -d " ")
		docker exec -it $CONTAINER_ID /garage bucket allow --read --write --owner beep --key beep_admin > /dev/null
		echo "KEY_ID=$KEY_ID"
		echo "SECRET_KEY=$SECRET_KEY"
fi

