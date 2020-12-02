#include <iostream>
#include <cerrno>
#include <sys/time.h>
#include <cstring>

// https://stackoverflow.com/questions/2408976/struct-timeval-to-printable-format

using namespace std;

int main() {
    std::cout << "Hello, World!" << std::endl;
    struct timeval tv{};
    struct timezone tz{};
    time_t nowtime;
    struct tm *nowtm;
    char tmbuf[86], buf[86];
    int status;

    status = gettimeofday(&tv, &tz);
    nowtime = tv.tv_sec;
    nowtm = localtime(&nowtime);
//    status = gettimeofday(&tv, &tz);
    strftime(tmbuf, sizeof tmbuf, "%Y-%m-%d %H:%M:%S", nowtm);
    snprintf(buf, sizeof buf, "%s.%06ld", tmbuf, tv.tv_usec);
    cout << status << " " << tmbuf << endl << buf << endl;

    gettimeofday(&tv, NULL);
    tv.tv_sec = tv.tv_sec + 666;
    status = settimeofday(&tv, &tz);
    if (status == 0) {
        cout << "Successfully set time" << endl;
    } else {
        std::cout << "log(-1) failed: " << std::strerror(errno) << '\n';

    }

    status = gettimeofday(&tv, NULL);
    nowtime = tv.tv_sec;
    nowtm = localtime(&nowtime);
//    status = gettimeofday(&tv, &tz);
    strftime(tmbuf, sizeof tmbuf, "%Y-%m-%d %H:%M:%S", nowtm);
    snprintf(buf, sizeof buf, "%s.%06ld", tmbuf, tv.tv_usec);
    cout << status << " " << tmbuf << endl << buf << endl;


    return 0;
}
