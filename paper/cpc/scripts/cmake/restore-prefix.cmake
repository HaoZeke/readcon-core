# project() in an old cmake_minimum_required resets the prefix to /usr/local.
if(READCON_SAVED_INSTALL_PREFIX AND NOT READCON_SAVED_INSTALL_PREFIX STREQUAL "/usr/local")
  set(CMAKE_INSTALL_PREFIX "${READCON_SAVED_INSTALL_PREFIX}" CACHE PATH "" FORCE)
endif()
