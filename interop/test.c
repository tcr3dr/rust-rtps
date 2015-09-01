#include <stdio.h>

const char* rmw_get_implementation_identifier();

int main() {
	printf("version: %s\n", rmw_get_implementation_identifier());
	return 0;
}