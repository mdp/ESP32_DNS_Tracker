/*
  DNS.h Library to encode base32 into multiple DNS queries
*/

#ifndef DNS32_h
#define DNS32_h

#include "Arduino.h"
#include "stdint.h"

// We want to ensure the response fits entirely inside a single UDP packet of 512 bytes
#define DNS32_MAX_QUERY_SIZE 238 // 238 is the max
// Legal size limit per subdomain
#define DNS32_LABEL_SIZE 63
// 16 bytes for [version/lastFlag 1][index 1][id 13][checksum 1] header
#define DNS32_QUERY_OVERHEAD 16

class DNS32
{
  public:
    DNS32(char*);
    int getQueriesLen(char*, int dnsLen=DNS32_MAX_QUERY_SIZE);
    int writeQuery(int, char*, char*, char (&)[254], int dnsLen=DNS32_MAX_QUERY_SIZE);
    static const char RFC4648_ALPHABET[33];

  private:
    char* _domain;
    int   _free_space;
};

#endif
