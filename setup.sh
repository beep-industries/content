#!/bin/bash

CONTAINER_ID=$(docker container list -a --format "{{.ID}} {{.Image}}" | grep "dxflrs/garage" | tr " " "," | sed 's/,.*//g')

NODE_ID=$(docker exec -t $CONTAINER_ID /garage node id)
NODE_ID="${NODE_ID%%@*}"

CLUSTER_LAYOUT=$(docker exec -t $CONTAINER_ID /garage layout show | grep "Current cluster layout version: 0")

if [ "$CLUSTER_LAYOUT" != "" ]; then
		docker exec -t $CONTAINER_ID /garage layout assign -z dc1 -c 1G $NODE_ID > /dev/null
		docker exec -t $CONTAINER_ID /garage layout apply --version 1 > /dev/null
fi

BUCKETS=("beep" "test")

for bucket in ${BUCKETS[@]}; do
		IS_BUCKET_PRESENT=$(docker exec -t $CONTAINER_ID /garage bucket list | grep "$bucket")
		if [ "$IS_BUCKET_PRESENT" == "" ]; then
			docker exec -t $CONTAINER_ID /garage bucket create $bucket > /dev/null
		fi
done

for bucket in ${BUCKETS[@]}; do
		IS_KEY_PRESENT=$(docker exec -t $CONTAINER_ID /garage key list | grep $bucket"_admin")
		if [ "$IS_KEY_PRESENT" == "" ]; then
				KEY_INFOS=$(docker exec -t $CONTAINER_ID /garage key create $bucket"_admin")
				KEY_ID=$(echo "$KEY_INFOS" | grep "Key ID" | cut -d ":" -f 2 | tr -d " ")
				SECRET_KEY=$(echo "$KEY_INFOS" | grep "Secret key" | cut -d ":" -f 2 | tr -d " ")
				docker exec -t $CONTAINER_ID /garage bucket allow --read --write --owner $bucket --key $bucket"_admin" > /dev/null
				if [ "$bucket" == "beep" ]; then
						echo "KEY_ID=$KEY_ID"
						echo "SECRET_KEY=$SECRET_KEY"
				fi

				if [ "$bucket" == "test" ]; then
						echo "TEST_KEY_ID=$KEY_ID"
						echo "TEST_SECRET_KEY=$SECRET_KEY"
				fi
		fi
done
