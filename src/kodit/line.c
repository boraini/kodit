#include <mem.h>

typedef char *VecItem;

struct Vec {
    VecItem *start;
    int length;
    int capacity;
}

void vec_init(Vec *vec) {
    vec->length = 0;
    vec->capacity = 10;
    vec->start = malloc(10);
}

void vec_push(Vec *vec, VecItem item) {
    if (vec->capacity == vec->length) {
        capacity *= 2;
        realloc(vec->start, capacity * sizeof(VecItem));
    }
    
    Vec *current = (vec->start + vec->length++);

    *current = item;
}

void vec_destroy(Vec *vec) {
    free(vec->start);
}

Vec decompose_line(char *line) {
    
}

void decompose_line_internal(char *line_left, int left_len, Vec items) {
    int start = strchr();
}