#!/bin/bash
set -e

update_env_var() {
	local key=$1
	local value=$2
	local env_file=".env"

	if [ ! -f "$env_file" ]; then
		echo "$key=$value" >> "$env_file"
	elif grep -q "^${key}=" "$env_file"; then
		sed -i "s|^${key}=.*|${key}=${value}|" "$env_file"
	else
		echo "$key=$value" >> "$env_file"
	fi
}

create_buckets() {
		BUCKETS=("beep" "test")
		for bucket in ${BUCKETS[@]}; do
				IS_BUCKET_PRESENT="$(docker exec -t $CONTAINER_ID /garage bucket list | grep "$bucket" | wc -l)"
				if [ "$IS_BUCKET_PRESENT" == "0" ]; then
					docker exec -t $CONTAINER_ID /garage bucket create $bucket > /dev/null
				fi
		done
}

function create_keys {
		BUCKETS=("beep" "test")
		for bucket in ${BUCKETS[@]}; do
				IS_KEY_PRESENT=$(docker exec -t $CONTAINER_ID /garage key list | grep $bucket"_admin" | wc -l)
				if [ "$IS_KEY_PRESENT" == "0" ]; then
						KEY_INFOS=$(docker exec -t $CONTAINER_ID /garage key create $bucket"_admin")
						KEY_ID=$(echo "$KEY_INFOS" | grep "Key ID" | cut -d ":" -f 2 | tr -d " ")
						SECRET_KEY=$(echo "$KEY_INFOS" | grep "Secret key" | cut -d ":" -f 2 | tr -d " ")
						docker exec -t $CONTAINER_ID /garage bucket allow --read --write --owner $bucket --key $bucket"_admin" > /dev/null
						if [ "$bucket" == "beep" ]; then
								if [ "$WRITE_ENV" == "true" ]; then
										update_env_var "KEY_ID" "$KEY_ID"
										update_env_var "SECRET_KEY" "$SECRET_KEY"
								fi
								echo "KEY_ID=$KEY_ID"
								echo "SECRET_KEY=$SECRET_KEY"
						fi

						if [ "$bucket" == "test" ]; then
								if [ "$WRITE_ENV" == "true" ]; then
										update_env_var "TEST_KEY_ID" "$KEY_ID"
										update_env_var "TEST_SECRET_KEY" "$SECRET_KEY"
								fi
								echo "TEST_KEY_ID=$KEY_ID"
								echo "TEST_SECRET_KEY=$SECRET_KEY"
						fi
				fi
		done
}

function setup_s3 {
		docker container list
		CONTAINER_ID=$(docker container list -a --format "{{.ID}} {{.Image}}" | grep "dxflrs/garage" | tr " " "," | sed 's/,.*//g')
		NODE_ID=$(docker exec -t $CONTAINER_ID /garage node id)
		NODE_ID="${NODE_ID%%@*}"

		CLUSTER_LAYOUT=$(docker exec -t $CONTAINER_ID /garage layout show | grep "Current cluster layout version: 0")

		if [ "$CLUSTER_LAYOUT" != "" ]; then
				docker exec -t $CONTAINER_ID /garage layout assign -z dc1 -c 1G $NODE_ID 1> /dev/null
				docker exec -t $CONTAINER_ID /garage layout apply --version 1 1> /dev/null
		fi

		
		create_buckets
		create_keys
}


function wait_until_s3_up {
	CONTAINER_ID=$(docker container list -a --format "{{.ID}} {{.Image}}" | grep "dxflrs/garage" | tr " " "," | sed 's/,.*//g')

	MAX_RETRIES=30
	RETRY_COUNT=0

	while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
		if docker exec -t $CONTAINER_ID /garage status &> /dev/null; then
			return 0
		fi

		RETRY_COUNT=$((RETRY_COUNT + 1))
		sleep 1
	done

	return 1
}

function setup_signing_key {
		SIGNING_KEY=$(cat /dev/random | head -c 20 | base64)
		echo "SIGNING_KEY=$SIGNING_KEY"
		if [ "$WRITE_ENV" == "true" ]; then
				update_env_var "SIGNING_KEY" "$SIGNING_KEY"
		fi
}

function reset {
		docker compose --ansi never down -v > /dev/null
		docker compose --ansi never up -d > /dev/null
}

case "$2" in
		"env")
				WRITE_ENV=true
				;;
				
		*)
				WRITE_ENV=false
				;;
esac

case "$1" in
		"reset")
				reset 
				wait_until_s3_up
				setup_s3
				setup_signing_key
				;;

		"setup-s3")
				setup_s3 ;;

		"gen-key")
				setup_signing_key ;;

		*)
				echo "Unknown action" ;;

esac
