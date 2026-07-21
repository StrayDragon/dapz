#include <stdio.h>

int bug(void) {
    int xs[] = {1, 2, 0};
    return xs[0] + xs[1];
}

int main(void) {
    printf("start\n");
    bug();
    printf("end\n");
    return 0;
}
