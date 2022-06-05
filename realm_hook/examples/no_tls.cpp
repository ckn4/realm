#include "../realm.h"


uint32_t realm_first_pkt_len()
{
    return 4;
};


int32_t realm_decide_remote_idx(int32_t idx, const char *buf)
{
    if (buf[0] != 0x16 || buf[1] != 0x03 || buf[2] != 0x01 ) {
        return -1;
    } else {
        return 0;
    }
};
