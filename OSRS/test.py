from numpy import isin


a = [1,2,3]
d = {'a':1, 'b':2, 'c':3}


print(type(a))

x = type(a) == list # True
y = isinstance(a, list) # True
z = type(d) == dict
w = isinstance(d, dict)

print(x, y, z, w)


class A:
    def __init__(self):
        self.a = 1


class B(A):
    def __init__(self):
        super().__init__()
        self.b = 2

print(isinstance(A(), A)) # True
print(type(A()) == A) # False

print(isinstance(B(), A)) # True
print(type(B()) == A) # False
