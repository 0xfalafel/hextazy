#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(int argc, char *argv[]) {

  if (argc < 2) {
    printf("Please enter a password.\n");
    printf("\n");
    printf("Usage: %s [password]", argv[0]);
    exit(0);
  }

  if (strcmp("S3cr3t", argv[1]) == 0) {
    printf("Congrats you found the password!");
  } else {
    printf("Wrong password");
  }
}
