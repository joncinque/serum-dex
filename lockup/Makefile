include ../Makefile

LIB_NAME=serum_lockup_program

.PHONY: test deploy-whitelist-program test-program

test: deploy-whitelist-program deploy-super test-program
	@ # no-op

deploy-whitelist-program:
	$(eval TMP=$(shell make -s -C tests/stake deploy))
	$(eval TEST_WHITELIST_PROGRAM_ID=$(shell echo $(TMP) | sed 's/.*{programId: \(.*\)}.*/\1/g'))

test-program:
	make TEST_PROGRAM_ID=$(TEST_PROGRAM_ID) TEST_WHITELIST_PROGRAM_ID=$(TEST_WHITELIST_PROGRAM_ID) test-program-super
