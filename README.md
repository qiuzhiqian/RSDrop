# protocol
# discovery
sender:
{
    device:{
        "name":"XML-XIAMENGLIANG",
        "type":"Deepin",
        "id":
    },
    "port":37850,
    "request":true
}

receiver:
{
    device:{
        "name":"XML-XIAMENGLIANG",
        "type":"Deepin",
        "id":
    },
    "port":37850,
    "request":false
}

# send
## handshake

### public key
sender:
{
    "type": "rsa"
    "data": xxxxxxxxxx
}

receiver:
{
    "type": "rsa"
    "data": xxxxxxxxxx
}

### File Meta data
sender:
{
    "files":[
        {
            "filename":"example.txt"
            "size":
            "verity": {
                "type":"md5"
                "data":"xxxxxxx"
            }
        }
    ]
}

receiver:
ack u8(1)

sender:
all files raw data