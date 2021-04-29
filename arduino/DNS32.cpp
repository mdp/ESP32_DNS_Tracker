#include "Arduino.h"
#include "DNS32.h"
#include "stdint.h"
#include "Math.h"

const int MAX_DNS_NAME_LEN = DNS32_MAX_QUERY_SIZE;
const int OVERHEAD = DNS32_QUERY_OVERHEAD;
const int DNS_QUERY_SIZE = MAX_DNS_NAME_LEN + 1; // Null Term
const int LABEL_MAX = DNS32_LABEL_SIZE; // Limit to label(subdomain) size to legal limit

const char DNS32::RFC4648_ALPHABET[33] = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

DNS32::DNS32(char* domain) 
{
  _domain = domain;
}

int ceil(float f)
{
  return int(std::ceil(f));
}

int freeSpacePerQuery(char* domain, int dnsLen)
{
  int limit = dnsLen - strlen(domain) - OVERHEAD;
  // Each part(subdomain) has one period, so free space is actually
  return limit - ceil(float(limit) / float(LABEL_MAX));
}

char checksum(char* in, int len)
{
  byte check = 0x00;
  for (int i=0; i < len; i++) {
    byte ch = (byte)in[i];
    if ((ch >= 0x41 && ch <= 0x5A) || (ch >= 0x61 && ch <= 0x7A)) { ch = ((ch & 0x1F) - 1); }
    else if (ch >= 0x32 && ch <= 0x37) { ch -= (0x32 - 26); }
    check = check ^ ch;
  }
  return DNS32::RFC4648_ALPHABET[check];
}

int DNS32::getQueriesLen(char* in, int dnsLen)
{
  int len = strlen(in);
  int freeSpace = freeSpacePerQuery(_domain, dnsLen);
  
  return ceil(float(len) / float(freeSpace));
}

// Returns length of string if successful, otherwise 0 if it's complete
int DNS32::writeQuery(int idx, char* id, char* in, char (&out)[254], int dnsLen)
{

  int inLen = strlen(in);
  bool last = false;
  
  Serial.printf("ID %.*s\n", 13, id);

  int freeSpace = freeSpacePerQuery(_domain, dnsLen);
  int startIdx = idx*freeSpace;
  if (startIdx >= inLen)
  {
    // Nothing left to output
    return 0;
  }
  
  int endIdx = startIdx + freeSpace;
  if (endIdx >= inLen)
  {
    endIdx = strlen(in);
    last = true;
  }
  
  int i = startIdx; // Input index
  int j = 0; // Output index including the additional '.'s


  // Build the headers
  out[j] = last ? 'B' : 'A'; j++;

  out[j] = RFC4648_ALPHABET[idx]; j++;

  while(j<15)
  {
    out[j] = id[j-2];
    j++;
  }

  out[j] = checksum(out, 15); j++; //Add a quick checksum to the preamble


  // Split out the text into chunks of [label].[label] with a max size each
  int p = 0; // Needed to track periods for chunking correctly
  while(i < endIdx) {
    Serial.printf("%c", (char)in[i]);
    if ((j - p) % LABEL_MAX == 0) {
      out[j] = (char)'.';
      j++; p++;
    }
    out[j] = (char)in[i];
    i++; j++;
  }

  // Append final '.' before domain;
  out[j] = (char)'.';
  j++;

  // Append domain
  for (int k = 0; k < strlen(_domain); k++)
  {
    out[j] = (char)_domain[k];
    j++;
  }
  out[j] = '\0';
  j++;
  
  return j;
}
