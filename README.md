# protocol
## discovery
```
sender:
{
    device:{
        "name":"XML-XIAMENGLIANG",
        "type":"linux",
        "id": "1678245913231837-368178"
    },
    "port":37850,
    "request":true
}
```

receiver:
```
{
    device:{
        "name":"XML-XIAMENGLIANG",
        "type":"Linux",
        "id": "1678245915970841-24082"
    },
    "port":37850,
    "request":false
}
```

## handshake

### public key
sender:
```
{
    "type": "rsa"
    "data": xxxxxxxxxx
}
```

receiver:
```
{
    "type": "rsa"
    "data": xxxxxxxxxx
}
```

### File Meta data
```
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
```

receiver:
ack u8(1)

## send file
sender:
all files raw data(byte stream)