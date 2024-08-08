import random
import time
import json
import os
import gc
import re
import secrets
import sys

config = {
    "main_coins_num": 22,
    "coins": ["BTC","ETH","USDT","USDC","XRP","DOGE","SOL","OKB","APT","DASH","DOT","ELF","EOS","ETC","FIL","LINK","LTC","OKT","PEOPLE","TON","TRX","UNI","1INCH","AAVE","ADA","AGLD","AIDOGE","AKITA","ALGO","ALPHA","ANT","APE","API3","AR","ARB","ATOM","AVAX","AXS","BABYDOGE","BADGER","BAL","BAND","BAT","BCH","BETH","BICO","BLUR","BNB","BNT","BSV","BTM","BZZ","CEL","CELO","CELR","CETUS","CFX","CHZ","CLV","COMP","CONV","CORE","CQT","CRO","CRV","CSPR","CVC","DOME","DORA","DYDX","EFI","EGLD","ENJ","ENS","ETHW","FITFI","FLM","FLOKI","FLOW","FTM","GALA","GFT","GLMR","GMT","GMX","GODS","GRT","HBAR","ICP","IMX","IOST","IOTA","JST","KISHU","KLAY","KNC","KSM","LAT","LDO","LON","LOOKS","LPT","LRC","LUNA","LUNC","MAGIC","MANA","MASK","MATIC","MINA","MKR","NEAR","NEO","NFT","OMG","ONT","OP","PEPE","PERP","QTUM","RDNT","REN","RSR","RSS3","RVN","SAND","SHIB","SKL","SLP","SNT","SNX","STARL","STORJ","STX","SUI","SUSHI","SWEAT","SWRV","THETA","TRB","TUSD","UMA","USTC","WAVES","WOO","XCH","XLM","XMR","XTZ","YFI","YFII","YGG","ZEC","ZEN","ZIL","ZRX"]
}    
coins = config["coins"]
coins_len = len(coins)
print(f"coins len: {coins_len}")

def generate_random_hex_string(length):
    # Each byte is represented by 2 hex characters
    num_bytes = (length + 1) // 2  # Calculate number of bytes needed
    random_bytes = secrets.token_bytes(num_bytes)
    hex_string = random_bytes.hex()
    return hex_string[:length]  # Truncate to the desired length


def init_user_data( batch_index, batch_size):

    data = []

    for i in range(batch_size):
        items = {"id" : generate_random_hex_string(64)}
        for coin in coins:
            items[coin] = str(random.randrange(2**32)//coins_len)
        data.append(items)
    current_working_directory = os.getcwd()
    user_data_path = os.path.join(current_working_directory, "test-data/user-data/batch" + str(batch_index) + ".json")
    with open(user_data_path, "w") as f:
        json.dump(data, f)
    return

if __name__ == '__main__':
    # Check if at least one argument is provided
    if len(sys.argv) < 2:
        print("Usage: python3 gen_test_data.py <num_of_docs> <accounts_per_doc>")
        sys.exit()

    # Retrieve and parse the command-line arguments
    try:
        int_args = [int(arg) for arg in sys.argv[1:]]
    except ValueError:
        print("Please provide integer arguments only.")
        sys.exit()

    num_of_docs = int_args[0]
    accounts_per_doc = int_args[1]
    print(f"generate user for {num_of_docs} docs, with {accounts_per_doc} accounts per doc")
    for i in range(num_of_docs):
        print(f"generate user for {i}-th doc")
        init_user_data(i, accounts_per_doc)
