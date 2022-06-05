#!/usr/bin/env bash
set -e

if [ "$#" -ne 1 ]; then
	echo "Please provide the number of initial validators!"
	exit 1
fi

if [ -z "$SECRET" ]; then
	echo "SECRET Empty!"
	exit 1
fi

generate_account_id() {
	subkey inspect -n cord ${3:-} ${4:-} "$SECRET//$1//$2" | grep "Account ID" | awk '{ print $3 }'
}

generate_address() {
	subkey inspect -n cord ${3:-} ${4:-} "$SECRET//$1//$2" | grep "SS58 Address" | awk '{ print $3 }'
}

generate_public_key() {
	subkey inspect -n cord ${3:-} ${4:-} "$SECRET//$1//$2" | grep "Public" | awk '{ print $4 }'
}

generate_address_and_public_key() {
	ADDRESS=$(generate_address $1 $2 $3)
	PUBLIC_KEY=$(generate_public_key $1 $2 $3)

	printf "//$ADDRESS\nhex![\"${PUBLIC_KEY#'0x'}\"].unchecked_into(),"
}

generate_address_and_account_id() {
	ACCOUNT=$(generate_account_id $1 $2 $3)
	ADDRESS=$(generate_address $1 $2 $3)
	if ${4:-false}; then
		INTO="unchecked_into"
	else
		INTO="into"
	fi

	printf "//$ADDRESS\nhex![\"${ACCOUNT#'0x'}\"].$INTO(),"
}

V_NUM=$1

AUTHORITIES="\nInitial Authorities \n"
AUTHORITIES_RPC="\nInitial Authorities (RPC) \n"
AUTHORITY_ACCOUNTS="\nInitial Authorities (Controller Accounts) (\n"

for i in $(seq 1 $V_NUM); do
	AUTHORITY_ACCOUNTS+="$(generate_address_and_account_id $i controller)\n"

	AUTHORITIES+="(\n"
	AUTHORITIES+="$(generate_address_and_account_id $i stash)\n"
	AUTHORITIES+="$(generate_address_and_account_id $i controller)\n"
	AUTHORITIES+="$(generate_address_and_account_id $i babe '--scheme sr25519' true)\n"
	AUTHORITIES+="$(generate_address_and_account_id $i grandpa '--scheme ed25519' true)\n"
	AUTHORITIES+="$(generate_address_and_account_id $i im_online '--scheme sr25519' true)\n"
	AUTHORITIES+="$(generate_address_and_account_id $i para_validator '--scheme sr25519' true)\n"
	AUTHORITIES+="$(generate_address_and_account_id $i para_assignment '--scheme sr25519' true)\n"
	AUTHORITIES+="$(generate_address_and_account_id $i authority_discovery '--scheme sr25519' true)\n"
	AUTHORITIES+="),\n"
	AUTHORITIES_RPC+="//$(generate_address $i controller) (\n"
	AUTHORITIES_RPC+="key type: babe\n"
	AUTHORITIES_RPC+="suri: $SECRET//$i//babe\n"
	AUTHORITIES_RPC+="public key: $(generate_account_id $i babe '--scheme sr25519')\n"
	AUTHORITIES_RPC+="key type: gran\n"
	AUTHORITIES_RPC+="suri: $SECRET//$i//grandpa\n"
	AUTHORITIES_RPC+="public key: $(generate_account_id $i grandpa '--scheme ed25519')\n"
	AUTHORITIES_RPC+="),\n"
done
AUTHORITY_ACCOUNTS+="),\n"

printf "$AUTHORITIES"
printf "$AUTHORITY_ACCOUNTS"
printf "$AUTHORITIES_RPC"